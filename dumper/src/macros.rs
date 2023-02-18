#[macro_export]
macro_rules! generate_gobjects_static_classes {
    ($($fname:ident, $fullname:literal),* $(,)?) => {
        #[allow(dead_code)]
        impl GObjects {
            $(
                pub fn $fname(&self, info: &Info) -> Result<Ptr> {
                    use once_cell::sync::OnceCell;

                    static CLASS: OnceCell<Ptr> = OnceCell::new();
                    CLASS
                        .get_or_try_init(|| {
                            self.find_by_full_name(info, $fullname)
                                .map(|obj| obj.expect(concat!($fullname, " is missing")))
                        })
                        .copied()
                }
            )*
        }
    };
}
