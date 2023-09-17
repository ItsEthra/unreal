#[macro_export]
macro_rules! impl_process_event_fns {
    (@fnbody $peidx:tt fn $name:ident $($arg_name:ident $arg_ty:ty),*; [$($ret_ty:ty)?] [$($body:tt)*]) => {
        pub fn $name(&mut self, $($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            $($body)*

            inner(<Self as $crate::UObjectExt>::as_uobject(self), $($arg_name),*)
        }
    };
    (@fnbody $peidx:tt static $name:ident $($arg_name:ident $arg_ty:ty),*; [$($ret_ty:ty)?] [$($body:tt)*]) => {
        pub fn $name($($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            $($body)*

            inner(<Self as $crate::UObjectLike>::static_class().cast(), $($arg_name),*)
        }
    };

    (@object fn $peidx:expr) => { <Self as $crate::UObjectExt<$peidx>>::as_uobject(self) };
    (@object static $peidx:expr) => { <Self as $crate::UObjectLike<$peidx>>::static_class() };

    (@retval $args:ident) => { () };
    (@retval $args:ident $ret_ty:ty) => { $args.__out };

    {
        [$target:ident, $peidx:expr]

        $(
            $kind:tt $fname:ident( $($arg_name:ident: $arg_ty:ty),* $(,)? )
            $(-> $ret_ty:ty)? = $fqn:expr;
        )*
    } => {
        #[allow(unused_variables, non_snake_case, dead_code)]
        impl $target {
            $(
                $crate::impl_process_event_fns!(@fnbody $peidx $kind $fname $($arg_name $arg_ty),*; [$($ret_ty)?] [
                    #[inline(always)]
                    fn inner(
                        mut obj: $crate::Ptr<$crate::UObject>,
                        $($arg_name: $arg_ty),*
                    ) $(-> $ret_ty)?
                    {
                        use $crate::Cache;

                        unsafe {
                            let function = (*$crate::DEFAULT_CACHE).lookup(&$crate::fqn!(#$fqn).hash());

                            #[repr(C)]
                            struct Args {
                                $($arg_name: $arg_ty,)*
                                $(__out: $ret_ty)?
                            }

                            let mut args: Args = ::core::mem::zeroed();
                            $(args.$arg_name = $arg_name;)*
                            obj.process_event($peidx, function, &mut args);
                            $crate::impl_process_event_fns!(@retval args $($ret_ty)?)
                        }
                    }
                ]);
            )*
        }
    };
}

#[macro_export]
macro_rules! impl_uobject_like {
    ($target:ty, $fqn:expr) => {
        unsafe impl $crate::UObjectLike for $target {
            fn static_class() -> $crate::Ptr<$crate::UClass> {
                use $crate::Cache;

                let class = (*$crate::DEFAULT_CACHE).lookup(&$crate::fqn!(#$fqn).hash());
                class.cast()
            }
        }
    };
}

mod inner {
    pub const PEIDX: usize = 15;
}

struct Foo;
impl_uobject_like!(Foo, "CoreUObject.Foo");
impl_process_event_fns!(
    [Foo, inner::PEIDX]


    fn Bar(a: i32, c: bool) -> u32 = "Bar";
    fn Tea(a: i32, b: u32) = "Tea";
    static Quz(a: i32, c: bool) -> u32 = "Quz";
);
