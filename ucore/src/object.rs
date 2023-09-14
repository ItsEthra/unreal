use crate::{assert_size, FName, GlobalContext, HashedFqn, Ptr};
use bitflags::bitflags;
use memflex::assert_offset;
use std::{
    iter::successors,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub struct FUObjectItem {
    pub object: *mut (),
    pub flags: u32,
    pub root_index: i32,
    pub serial: u32,
}
assert_size!(FUObjectItem, 0x18);

#[derive(Debug)]
#[allow(dead_code)]
pub struct FChunkedFixedUObjectArray {
    objects: *const *const FUObjectItem,
    preallocated: *const (),
    max_elems: u32,
    num_elems: u32,
    max_chunks: u32,
    num_chunks: u32,
}

impl FChunkedFixedUObjectArray {
    pub fn iter<const PEIDX: usize>(&self) -> impl Iterator<Item = Ptr<UObject<PEIDX>>> + '_ {
        (0..self.num_elems).flat_map(|i| self.nth(i)).fuse()
    }

    pub fn nth<const PEIDX: usize>(&self, idx: u32) -> Option<Ptr<UObject<PEIDX>>> {
        const NUM_ELEMS_PER_CHUNK: usize = 64 * 1024;

        let chunk_idx = idx as usize / NUM_ELEMS_PER_CHUNK;
        let array = GlobalContext::get().chunked_fixed_uobject_array();
        let item = unsafe {
            array
                .objects
                .add(chunk_idx)
                .read()
                .add(idx as usize % NUM_ELEMS_PER_CHUNK)
        };
        let object = NonNull::new(unsafe { (*item).object.cast() })?;
        Some(Ptr(object))
    }

    pub fn by_fqn<const PEIDX: usize>(&self, hash: HashedFqn) -> Option<Ptr<UObject<PEIDX>>> {
        GlobalContext::get()
            .chunked_fixed_uobject_array()
            .iter()
            .find(|obj| obj.eq_fqn(hash))
    }
}

#[allow(dead_code)]
pub struct UClass<const PEIDX: usize> {
    object: UObject<PEIDX>,
    next: *const (),
    _pad_0x40: [u8; 0x10],
    super_struct: Option<Ptr<Self>>,
}
assert_offset!(UClass<0>, super_struct, 0x40);

impl<const PEIDX: usize> UClass<PEIDX> {
    pub fn is(&self, class: Ptr<Self>) -> bool {
        successors(Some(Ptr::from_ref(self)), |class| class.super_struct).any(|ptr| ptr == class)
    }

    #[inline]
    pub fn super_struct(&self) -> Option<Ptr<Self>> {
        self.super_struct
    }
}

impl<const PEIDX: usize> Deref for UClass<PEIDX> {
    type Target = UObject<PEIDX>;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl<const PEIDX: usize> DerefMut for UClass<PEIDX> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}

/// # Safety
/// * Must only implemented for unreal engine UObjects
pub unsafe trait UObjectLike<const PEIDX: usize>: Sized {
    fn static_class() -> Ptr<UClass<PEIDX>>;
}

pub unsafe trait UObjectExt<const PEIDX: usize>: UObjectLike<PEIDX> {
    fn is<T: UObjectLike<PEIDX>>(&self) -> bool;

    fn cast_ref<T: UObjectLike<PEIDX>>(&self) -> Option<&T>;
    unsafe fn cast_ref_unchecked<T: UObjectLike<PEIDX>>(&self) -> &T;

    fn cast_mut<T: UObjectLike<PEIDX>>(&mut self) -> Option<&mut T>;
    unsafe fn cast_mut_unchecked<T: UObjectLike<PEIDX>>(&mut self) -> &mut T;

    fn from_uobject(object: Ptr<UObject<PEIDX>>) -> Option<Ptr<Self>>;
    unsafe fn from_uobject_unchecked(object: Ptr<UObject<PEIDX>>) -> Ptr<Self>;

    fn as_uobject(&self) -> Ptr<UObject<PEIDX>>;
}

unsafe impl<const PEIDX: usize, O: UObjectLike<PEIDX>> UObjectExt<PEIDX> for O {
    fn is<T: UObjectLike<PEIDX>>(&self) -> bool {
        self.as_uobject().class.is(T::static_class())
    }

    fn cast_ref<T: UObjectLike<PEIDX>>(&self) -> Option<&T> {
        if self.is::<T>() {
            Some(unsafe { &*(self as *const Self as *const T) })
        } else {
            None
        }
    }

    unsafe fn cast_ref_unchecked<T: UObjectLike<PEIDX>>(&self) -> &T {
        unsafe { &*(self as *const Self as *const T) }
    }

    fn cast_mut<T: UObjectLike<PEIDX>>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            Some(unsafe { &mut *(self as *mut Self as *mut T) })
        } else {
            None
        }
    }

    unsafe fn cast_mut_unchecked<T: UObjectLike<PEIDX>>(&mut self) -> &mut T {
        unsafe { &mut *(self as *mut Self as *mut T) }
    }

    fn from_uobject(object: Ptr<UObject<PEIDX>>) -> Option<Ptr<Self>> {
        if object.class.is(Self::static_class()) {
            Some(Ptr(object.0.cast::<Self>()))
        } else {
            None
        }
    }

    unsafe fn from_uobject_unchecked(object: Ptr<UObject<PEIDX>>) -> Ptr<O> {
        Ptr(object.0.cast::<Self>())
    }

    fn as_uobject(&self) -> Ptr<UObject<PEIDX>> {
        unsafe {
            Ptr(NonNull::new_unchecked(
                (self as *const Self).cast::<UObject<PEIDX>>().cast_mut(),
            ))
        }
    }
}

// PEIDX = Process Event Index
#[repr(C)]
pub struct UObject<const PEIDX: usize> {
    vmt: *const (),
    flags: ObjectFlags,
    index: u32,
    class: Ptr<UClass<PEIDX>>,
    name: FName,
    outer: Option<Ptr<Self>>,
}
assert_size!(UObject<0>, 0x28);

unsafe impl<const PEIDX: usize> Send for UObject<PEIDX> {}
unsafe impl<const PEIDX: usize> Sync for UObject<PEIDX> {}

impl<const PEIDX: usize> UObject<PEIDX> {
    #[inline]
    pub fn get_by_fqn(hash: HashedFqn) -> Option<Ptr<Self>> {
        GlobalContext::get()
            .chunked_fixed_uobject_array()
            .by_fqn(hash)
    }

    #[inline]
    pub fn flags(&self) -> ObjectFlags {
        self.flags
    }

    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }

    #[inline]
    pub fn outer(&self) -> Option<Ptr<Self>> {
        self.outer
    }

    #[inline]
    pub fn class(&self) -> Ptr<UClass<PEIDX>> {
        self.class
    }

    #[inline]
    pub fn name(&self) -> FName {
        self.name
    }

    pub fn eq_fqn(&self, hash: HashedFqn) -> bool {
        let namepool = GlobalContext::get().name_pool();
        successors(Some(Ptr::from_ref(self)), |obj| obj.outer)
            .map(|obj| namepool.resolve(obj.name.index()))
            .enumerate()
            .all(|(i, entry)| hash.0[i] == entry.hash())
    }

    /// # Safety
    /// * Process event function index (`PEIDX`) was set correctly and object has a valid VMT pointer.
    pub unsafe fn process_event<Args>(&mut self, function: Ptr<Self>, args: *mut Args) {
        self.vmt
            .cast::<extern "C" fn(&mut Self, Ptr<Self>, *mut Args)>()
            .add(PEIDX)
            .read()(self, function, args);
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ObjectFlags : u32 {
        const NoFlags = 0x00000000;
        const Public = 0x00000001;
        const Standalone = 0x00000002;
        const MarkAsNative = 0x00000004;
        const Transactional = 0x00000008;
        const ClassDefaultObject = 0x00000010;
        const ArchetypeObject = 0x00000020;
        const Transient = 0x00000040;
        const MarkAsRootSet = 0x00000080;
        const TagGarbageTemp = 0x00000100;
        const NeedInitialization = 0x00000200;
        const NeedLoad = 0x00000400;
        const KeepForCooker = 0x00000800;
        const NeedPostLoad = 0x00001000;
        const NeedPostLoadSubobjects = 0x00002000;
        const NewerVersionExists = 0x00004000;
        const BeginDestroyed = 0x00008000;
        const FinishDestroyed = 0x00010000;
        const BeingRegenerated = 0x00020000;
        const DefaultSubObject = 0x00040000;
        const WasLoaded = 0x00080000;
        const TextExportTransient = 0x00100000;
        const LoadCompleted = 0x00200000;
        const InheritableComponentTemplate = 0x00400000;
        const DuplicateTransient = 0x00800000;
        const StrongRefOnFrame = 0x01000000;
        const NonPIEDuplicateTransient = 0x02000000;
        const Dynamic = 0x04000000;
        const WillBeLoaded = 0x08000000;
        const HasExternalPackage = 0x10000000;
        const PendingKill = 0x20000000;
        const Garbage = 0x40000000;
        const AllocatedInSharedPage	= 0x80000000;
    }
}
