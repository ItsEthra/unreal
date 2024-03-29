use crate::{FName, GlobalContext, HashedFqn, Ptr};
use bitflags::bitflags;
use memflex::assert_size;
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
    pub fn iter(&self) -> impl Iterator<Item = Ptr<UObject>> + '_ {
        (0..self.num_elems).flat_map(|i| self.nth(i)).fuse()
    }

    pub fn nth(&self, idx: u32) -> Option<Ptr<UObject>> {
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

    pub fn by_fqn(&self, hash: HashedFqn) -> Option<Ptr<UObject>> {
        GlobalContext::get()
            .chunked_fixed_uobject_array()
            .iter()
            .find(|obj| obj.eq_fqn(hash))
    }
}

#[allow(dead_code)]
pub struct UClass {
    object: UObject,
    next: *const (),
    _pad_0x40: [u8; 0x10],
    super_struct: Option<Ptr<Self>>,
}

impl UClass {
    pub fn is(&self, class: Ptr<Self>) -> bool {
        successors(Some(Ptr::from_ref(self)), |class| class.super_struct).any(|ptr| ptr == class)
    }

    #[inline]
    pub fn super_struct(&self) -> Option<Ptr<Self>> {
        self.super_struct
    }
}

impl Deref for UClass {
    type Target = UObject;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl DerefMut for UClass {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}

/// UObject trait for unreal engine objects
/// # Safety
/// * Must only implemented for unreal engine UObjects
pub unsafe trait UObjectLike: Sized {
    fn static_class() -> Ptr<UClass>;
}

/// Helper functions for convenient use of UObjects
/// # Safety
/// See [`UObjectLike`]
pub unsafe trait UObjectExt: UObjectLike {
    fn is<T: UObjectLike>(&self) -> bool;

    fn cast_ref<T: UObjectLike>(&self) -> Option<&T>;
    /// # Safety
    /// * Inheritance rules apply
    unsafe fn cast_ref_unchecked<T: UObjectLike>(&self) -> &T;

    fn cast_mut<T: UObjectLike>(&mut self) -> Option<&mut T>;
    /// # Safety
    /// * Inheritance rules apply
    unsafe fn cast_mut_unchecked<T: UObjectLike>(&mut self) -> &mut T;

    fn from_uobject(object: Ptr<UObject>) -> Option<Ptr<Self>>;
    /// # Safety
    /// * Inheritance rules apply
    unsafe fn from_uobject_unchecked(object: Ptr<UObject>) -> Ptr<Self>;

    fn as_uobject(&self) -> Ptr<UObject>;
}

unsafe impl<O: UObjectLike> UObjectExt for O {
    fn is<T: UObjectLike>(&self) -> bool {
        self.as_uobject().class.is(T::static_class())
    }

    fn cast_ref<T: UObjectLike>(&self) -> Option<&T> {
        if self.is::<T>() {
            Some(unsafe { &*(self as *const Self as *const T) })
        } else {
            None
        }
    }

    unsafe fn cast_ref_unchecked<T: UObjectLike>(&self) -> &T {
        unsafe { &*(self as *const Self as *const T) }
    }

    fn cast_mut<T: UObjectLike>(&mut self) -> Option<&mut T> {
        if self.is::<T>() {
            Some(unsafe { &mut *(self as *mut Self as *mut T) })
        } else {
            None
        }
    }

    unsafe fn cast_mut_unchecked<T: UObjectLike>(&mut self) -> &mut T {
        unsafe { &mut *(self as *mut Self as *mut T) }
    }

    fn from_uobject(object: Ptr<UObject>) -> Option<Ptr<Self>> {
        if object.class.is(Self::static_class()) {
            Some(Ptr(object.0.cast::<Self>()))
        } else {
            None
        }
    }

    unsafe fn from_uobject_unchecked(object: Ptr<UObject>) -> Ptr<O> {
        Ptr(object.0.cast::<Self>())
    }

    fn as_uobject(&self) -> Ptr<UObject> {
        unsafe {
            Ptr(NonNull::new_unchecked(
                (self as *const Self).cast::<UObject>().cast_mut(),
            ))
        }
    }
}

// PEIDX = Process Event Index
#[repr(C)]
pub struct UObject {
    vmt: *const (),
    flags: ObjectFlags,
    index: u32,
    class: Ptr<UClass>,
    name: FName,
    outer: Option<Ptr<Self>>,
}
assert_size!(UObject, 0x28);

unsafe impl Send for UObject {}
unsafe impl Sync for UObject {}

impl UObject {
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
    pub fn class(&self) -> Ptr<UClass> {
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
    /// * Process event function `index` was set correctly and object has a valid VMT pointer.
    pub unsafe fn process_event<Args>(
        &mut self,
        index: usize,
        function: Ptr<Self>,
        args: *mut Args,
    ) {
        self.vmt
            .cast::<extern "C" fn(&mut Self, Ptr<Self>, *mut Args)>()
            .add(index)
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
