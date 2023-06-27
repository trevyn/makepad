#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}
