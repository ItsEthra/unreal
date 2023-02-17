pub trait UObject {
    const CLASS_NAME: &'static str;

    // TODO: Implement
    fn is<O: UObject>(&self) -> bool;

    // TODO: Implement
    fn cast_ref<O: UObject>(&self) -> Option<&O>;

    // TODO: Implement
    fn cast_mut<O: UObject>(&mut self) -> Option<&mut O>;
}
