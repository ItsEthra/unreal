use crate::{assert_size, FName, GlobalContext, Ptr};
use bitflags::bitflags;
use std::ptr::NonNull;

pub struct FUObjectItem {
    pub object: *const (),
    pub flags: u32,
    pub root_index: i32,
    pub serial: u32,
}
assert_size!(FUObjectItem, 0x18);

#[allow(dead_code)]
pub struct FChunkedFixedUObjectArray {
    objects: Ptr<Ptr<FUObjectItem>>,
    preallocated: Ptr<FUObjectItem>,
    max_elems: u32,
    num_elems: u32,
    max_chunks: u32,
    num_chunks: u32,
}

// PEIDX = Process Event Index
#[repr(C)]
pub struct UObject<const PEIDX: usize> {
    vmt: *const (),
    flags: ObjectFlags,
    index: u32,
    class: Ptr<Self>,
    name: FName,
    outer: Option<Ptr<Self>>,
}
assert_size!(UObject<0>, 0x28);

impl<const PEIDX: usize> memflex::Cast for UObject<PEIDX> {}

unsafe impl<const PEIDX: usize> Send for UObject<PEIDX> {}
unsafe impl<const PEIDX: usize> Sync for UObject<PEIDX> {}

impl<const PEIDX: usize> UObject<PEIDX> {
    pub fn get_by_index(idx: usize) -> Ptr<Self> {
        const NUM_ELEMS_PER_CHUNK: usize = 64 * 1024;

        let chunk_idx = idx / NUM_ELEMS_PER_CHUNK;
        let array = GlobalContext::get().chunked_fixed_uobject_array();
        let chunk = unsafe { array.objects.0.as_ptr().add(chunk_idx).read() };
        let objitem = unsafe { chunk.0.as_ptr().add(idx % NUM_ELEMS_PER_CHUNK) };
        let object = NonNull::new(unsafe { (*objitem).object.cast_mut().cast() }).unwrap();
        Ptr(object)
    }

    pub fn flags(&self) -> ObjectFlags {
        self.flags
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn class(&self) -> Ptr<Self> {
        self.class
    }

    pub fn name(&self) -> FName {
        self.name
    }

    pub unsafe fn process_event<Args>(&self, function: Ptr<Self>, args: *const Args) {
        self.vmt
            .cast::<fn(Ptr<Self>, *const Args)>()
            .add(PEIDX)
            .read()(function, args);
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
