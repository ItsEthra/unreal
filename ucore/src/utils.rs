use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[macro_export]
macro_rules! assert_size {
    ($target:path, $size:tt) => {
        const _: () = if core::mem::size_of::<$target>() != $size {
            panic!(concat!(
                "Size assertion failed! sizeof(",
                stringify!($target),
                ") != ",
                stringify!($size)
            ))
        } else {
            ()
        };
    };
}

#[macro_export]
macro_rules! impl_process_event_fns {
    { @retty } => { () };
    { @retty $ty:ty } => { $ty };
    { @retty $($ty:ty)* } => { ($($ty),*) };

    { @retval $args:ident } => { () };
    { @retval $args:ident $name:ident } => { $args.$name };
    { @retval $args:ident $($name:ident)* } => { ($($args.$name),*) };

    {
        [$target:ident, $peidx:expr],
        $(
            $vis:vis fn $name:ident($($arg_name:ident: $arg_ty:ty),* $(,)?) $(-> [$($ret_name:ident: $ret_ty:ty),* $(,)?])? = $index:expr
        );* $(;)?
    } => {
        impl $target {
            $(
                #[allow(non_snake_case)]
                $vis fn $name(&self, $($arg_name: $arg_ty),*) -> crate::impl_process_event_fns!( @retty $( $($ret_ty)* )? ) {
                    static mut FUNCTION: Option<Ptr<$crate::UObject<$peidx>>> = None;

                    unsafe {
                        if FUNCTION.is_none() {
                            FUNCTION = Some($crate::UObject::get_by_index($index));
                        }
                    }

                    #[repr(C)]
                    struct Args {
                        $($arg_name: $arg_ty,)*
                        $( $($ret_name: $ret_ty,)* )?
                    }

                    unsafe {
                        let args = Args {
                            $($arg_name,)*
                            $( $($ret_name: zeroed(),)* )?
                        };
                        let object = <Self as $crate::UObjectLike<$peidx>>::as_uobject(self);
                        object.process_event(*FUNCTION.as_ref().unwrap(), &args);
                        crate::impl_process_event_fns!( @retval args $( $($ret_name)* )? )
                    }
                }
            )*
        }
    };
}

pub struct Shrink<const SIZE: usize, T> {
    buf: [u8; SIZE],
    pd: PhantomData<T>,
}

impl<const SIZE: usize, T> Deref for Shrink<SIZE, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.buf.as_ptr().cast::<T>().as_ref().unwrap() }
    }
}

impl<const SIZE: usize, T> DerefMut for Shrink<SIZE, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.buf.as_mut_ptr().cast::<T>().as_mut().unwrap() }
    }
}

#[repr(transparent)]
pub struct Ptr<T: ?Sized>(pub NonNull<T>);

impl<T: ?Sized> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: ?Sized> Eq for Ptr<T> {}

impl<T: ?Sized> Ptr<T> {
    pub fn from_ref(r: &T) -> Self {
        unsafe { Self(NonNull::new_unchecked(r as *const T as _)) }
    }
}

impl<T: ?Sized> Clone for Ptr<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<T: ?Sized> Copy for Ptr<T> {}

unsafe impl<T: ?Sized> Send for Ptr<T> {}
unsafe impl<T: ?Sized> Sync for Ptr<T> {}

impl<T: ?Sized> Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T: ?Sized> From<NonNull<T>> for Ptr<T> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }
}

impl<T: ?Sized> Deref for Ptr<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Ptr<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}
