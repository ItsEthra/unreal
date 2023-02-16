pub trait UObject {
    const CLASS_NAME: &'static str;

    fn is<O: UObject>(&self) -> bool;
    fn cast_ref<O: UObject>(&self) -> Option<&O>;
    fn cast_mut<O: UObject>(&mut self) -> Option<&mut O>;
}
