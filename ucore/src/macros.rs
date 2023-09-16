#[macro_export]
macro_rules! impl_process_event_fns {
    (@makeresult $result:ident $name:ident: $ty:ty) => { };
    (@makeresult $result:ident $($name:ident: $ty:ty),*) => {
        #[allow(non_camel_case_types, dead_code)]
        pub struct $result {
            $(pub $name: $ty),*
        }
    };

    (@retty) => { () };
    (@retty $result:ident $name:ident: $ty:ty) => { $ty };
    (@retty $result:ident $($name:ident: $ty:ty),*) => { $result };

    (@fnbody $peidx:tt fn $name:ident $($arg_name:ident $arg_ty:ty),*; $($result:ident $($ret_name:ident $ret_ty:ty),*)? [$($body:tt)*]) => {
        pub fn $name(&mut self, $($arg_name: $arg_ty),*) -> $crate::impl_process_event_fns!(@retty $($result $($ret_name: $ret_ty),*)? ) {
            $($body)*

            inner(<Self as $crate::UObjectExt>::as_uobject(self), $($arg_name),*)
        }
    };
    (@fnbody $peidx:tt static $name:ident $($arg_name:ident $arg_ty:ty),*; $($result:ident $($ret_name:ident $ret_ty:ty),*)? [$($body:tt)*]) => {
        pub fn $name($($arg_name: $arg_ty),*) -> $crate::impl_process_event_fns!(@retty $($result $($ret_name: $ret_ty),*)? ) {
            $($body)*

            inner(<Self as $crate::UObjectLike>::static_class().cast(), $($arg_name),*)
        }
    };

    (@object fn $peidx:expr) => { <Self as $crate::UObjectExt<$peidx>>::as_uobject(self) };
    (@object static $peidx:expr) => { <Self as $crate::UObjectLike<$peidx>>::static_class() };

    (@retval $args:ident) => { () };
    (@retval $args:ident $result:ident $name:ident: $ty:ty) => { $args.$name };
    (@retval $args:ident $result:ident $($name:ident: $ty:ty),*) => { return $result {
        $($name: $args.$name),*
    } };

    {
        [$target:ident, $peidx:tt]

        $(
            $kind:tt $fname:ident( $($arg_name:ident: $arg_ty:ty),* $(,)? ) $(-> [$result:ident; $($ret_name:ident: $ret_ty:ty),* $(,)?] )? = $fqn:expr;
            { $($param_name:ident: $param_ty:ty),* $(,)? }
        )*
    } => {
        $($(
            $crate::impl_process_event_fns!(@makeresult $result $($ret_name: $ret_ty),* );
        )?)*

        #[allow(unused_variables, non_snake_case, dead_code)]
        impl $target {
            $(
                $crate::impl_process_event_fns!(@fnbody $peidx $kind $fname $($arg_name $arg_ty),*; $($result $($ret_name $ret_ty),*)? [
                    #[inline(always)]
                    fn inner(
                        mut obj: $crate::Ptr<$crate::UObject>,
                        $($arg_name: $arg_ty),*
                    )-> $crate::impl_process_event_fns!(@retty $($result $($ret_name: $ret_ty),*)? )
                    {
                        use $crate::Cache;

                        unsafe {
                            let function = (*$crate::DEFAULT_CACHE).lookup(&$crate::fqn!(#$fqn).hash());

                            #[repr(C)]
                            struct Args {
                                $($param_name: $param_ty),*
                            }

                            let mut args: Args = ::std::mem::zeroed();
                            $(args.$arg_name = $arg_name;)*
                            obj.process_event($peidx, function, &mut args);
                            $crate::impl_process_event_fns!(@retval args $($result $($ret_name: $ret_ty),*)?)
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

struct Foo;
impl_uobject_like!(Foo, "CoreUObject.Foo");
impl_process_event_fns!(
    [Foo, 0x4D]

    fn Bar(a: i32, c: bool) -> [Bar_Result; b: u32] = "Bar";
    { a: i32, b: u32, c: bool }

    static Quz(a: i32, c: bool) -> [Quz_Result; b: u32, d: bool] = "Quz";
    { a: i32, b: u32, c: bool, d: bool }

    fn Tea(a: i32, b: u32) = "Tea";
    { a: i32, b: u32 }
);
