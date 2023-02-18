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
