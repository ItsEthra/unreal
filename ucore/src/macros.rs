#[macro_export]
#[doc(hidden)]
macro_rules! impl_offset_members {
    ($target:ident, $($vs:vis $field:ident => $name:ident as $type:ty),*) => {
        impl<O: $crate::offsets::Offsets> $target<O> {
            $(
                $vs fn $name(&self) -> &$type {
                    use $crate::offsets::*;

                    unsafe {
                        (self as *const Self)
                            .cast::<u8>()
                            .add(O::$target::$field)
                            .cast::<$type>()
                            .as_ref()
                            .unwrap()
                    }
                }

                $crate::__paste! {
                    $vs fn [<$name _mut>](&mut self) -> &mut $type {
                        use $crate::offsets::*;

                        unsafe {
                            (self as *mut Self)
                                .cast::<u8>()
                                .add(O::$target::$field)
                                .cast::<$type>()
                                .as_mut()
                                .unwrap()
                        }
                    }
                }
            )*
        }
    };
}
