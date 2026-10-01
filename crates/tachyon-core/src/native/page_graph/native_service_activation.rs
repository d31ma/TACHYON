struct NativeServiceActivation {
    scope: NativeServiceScope,
    failure: core::cell::RefCell<Option<String>>,
    checkpoint: usize,
}

impl NativeServiceActivation {
    fn fail(&self, failure: String) {
        self.failure.replace(Some(failure));
    }
}
