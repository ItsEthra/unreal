#[macro_export]
macro_rules! impl_process_event_fns {
    { @retty } => { () };
    { @retty $ret_struct:ty; $ret_ty:ty } => { $ret_ty };
    { @retty $ret_struct:ty; $($ret_ty:ty)* } => { $ret_struct };

    { @retval $args:ident } => { };
    { @retval $args:ident $ret_struct:ident $ret_name:ident } => { return $args.$ret_name; };
    { @retval $args:ident $ret_struct:ident $($ret_name:ident)*} => {
        return $ret_struct {
            $($ret_name: $args.$ret_name,)*
        };
    };

    {
        [$target:ident, $peidx:expr],
        $(
            $vis:vis fn $name:ident($($arg_name:ident: $arg_ty:ty),* $(,)?) $(-> [<$ret_struct:ident> $($ret_name:ident: $ret_ty:ty),* ])? = $index:expr
        );* $(;)?
    } => {
        $(
            $(
                #[allow(dead_code, non_snake_case)]
                pub struct $ret_struct {
                    $(pub $ret_name: $ret_ty,)*
                }
            )?
        )*

        impl $target {
            $(
                #[allow(dead_code, non_snake_case)]
                $vis fn $name(&mut self, $($arg_name: $arg_ty),*) -> $crate::impl_process_event_fns!( @retty $( $ret_struct; $($ret_ty)* )? ) {
                    static mut FUNCTION: Option<Ptr<$crate::UObject<$peidx>>> = None;

                    unsafe {
                        if FUNCTION.is_none() {
                            FUNCTION = Some($crate::UObject::get_by_index($index));
                        }
                    }

                    #[repr(C)]
                    struct Args {
                        $( $arg_name: $arg_ty, )*
                        $( $( $ret_name: $ret_ty, )* )?
                    }

                    unsafe {
                        let mut args = Args {
                            $($arg_name,)*
                            $( $($ret_name: std::mem::zeroed(),)* )?
                        };
                        let mut object = <Self as $crate::UObjectLike<$peidx>>::as_uobject(self);
                        object.process_event(*FUNCTION.as_ref().unwrap(), &mut args);

                        $crate::impl_process_event_fns!( @retval args $( $ret_struct $($ret_name)* )? );
                    }
                }
            )*
        }
    };
    {
        [$target:ident, $peidx:expr],
        $(
            $vis:vis static fn $name:ident($($arg_name:ident: $arg_ty:ty),* $(,)?) $(-> [<$ret_struct:ident> $($ret_name:ident: $ret_ty:ty),* ])? = $index:expr
        );* $(;)?
    } => {
        $(
            $(
                #[allow(dead_code, non_snake_case)]
                pub struct $ret_struct {
                    $(pub $ret_name: $ret_ty,)*
                }
            )?
        )*

        impl $target {
            $(
                #[allow(dead_code, non_snake_case)]
                $vis fn $name($($arg_name: $arg_ty),*) -> $crate::impl_process_event_fns!( @retty $( $ret_struct; $($ret_ty)* )? ) {
                    static mut FUNCTION: Option<Ptr<$crate::UObject<$peidx>>> = None;

                    unsafe {
                        if FUNCTION.is_none() {
                            FUNCTION = Some($crate::UObject::get_by_index($index));
                        }
                    }

                    #[repr(C)]
                    struct Args {
                        $( $arg_name: $arg_ty, )*
                        $( $( $ret_name: $ret_ty, )* )?
                    }

                    unsafe {
                        let mut args = Args {
                            $($arg_name,)*
                            $( $($ret_name: std::mem::zeroed(),)* )?
                        };
                        let mut class = <Self as $crate::UObjectLike<$peidx>>::static_class();
                        class.process_event(*FUNCTION.as_ref().unwrap(), &mut args);

                        $crate::impl_process_event_fns!( @retval args $( $ret_struct $($ret_name)* )? );
                    }
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! impl_uobject_like {
    ($target:ty, $peidx:expr, $idx:expr) => {
        unsafe impl $crate::UObjectLike<{ $peidx }> for $target {
            const INDEX: u32 = $idx;
        }
    };
}
