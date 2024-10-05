pub mod x_stretcher;
pub mod y_stretcher;

pub trait Stretcher<T> {
    fn no_stretch() -> Self;

    fn stretches(&self) -> bool;

    fn irreversible(&self) -> bool;

    fn stretched(&self, item: &T) -> T;

    fn stretch(&self, item: &mut T);
}
