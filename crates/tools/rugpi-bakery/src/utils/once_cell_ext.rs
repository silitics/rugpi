use std::cell::OnceCell;

/// Extension trait for [`OnceCell`].
pub trait OnceCellExt<T> {
    /// Gets the contents of the cell or tries to initialize it.
    ///
    /// We can remove this once `get_or_try_init` lands in the standard library (see
    /// [#109737](https://github.com/rust-lang/rust/issues/109737)).
    fn try_get_or_init<E>(&self, init: impl FnOnce() -> Result<T, E>) -> Result<&T, E>;
}

impl<T> OnceCellExt<T> for OnceCell<T> {
    fn try_get_or_init<E>(&self, init: impl FnOnce() -> Result<T, E>) -> Result<&T, E> {
        if let Some(value) = self.get() {
            return Ok(value);
        }
        if self.set(init()?).is_err() {
            panic!("concurrent initialization of `OnceCell`");
        }
        Ok(self.get().unwrap())
    }
}
