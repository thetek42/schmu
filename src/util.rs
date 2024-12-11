pub struct CallOnDrop<T, F: FnOnce() -> T> {
    closure: Option<F>,
}

impl<T, F: FnOnce() -> T> CallOnDrop<T, F> {
    pub fn new(f: F) -> Self {
        Self { closure: Some(f) }
    }
}

impl<T, F: FnOnce() -> T> Drop for CallOnDrop<T, F> {
    fn drop(&mut self) {
        if let Some(f) = self.closure.take() {
            _ = f();
        }
    }
}
