struct NativeServiceNode {
    value: Box<dyn std::any::Any>,
    capabilities: std::rc::Rc<NativeCapabilityScope>,
    start: Box<dyn Fn() -> Result<(), String>>,
    stop: Box<dyn Fn() -> Result<(), String>>,
    started: core::cell::Cell<bool>,
    disposed: core::cell::Cell<bool>,
}

impl NativeServiceNode {
    fn new<T: Clone + 'static>(
        value: T,
        capabilities: std::rc::Rc<NativeCapabilityScope>,
        definition: &NativeServiceDefinition<T>,
    ) -> Self {
        let beginning = value.clone();
        let retained = value.clone();
        let start = definition.start;
        let stop = definition.stop;
        Self {
            value: Box::new(value),
            capabilities,
            start: Box::new(move || start(&beginning)),
            stop: Box::new(move || stop(&retained)),
            started: core::cell::Cell::new(false),
            disposed: core::cell::Cell::new(false),
        }
    }

    fn value<T: Clone + 'static>(&self) -> Result<T, String> {
        self.value
            .downcast_ref::<T>()
            .cloned()
            .ok_or_else(|| String::from("Native provider type mismatch."))
    }

    fn start_dependency(&self, deferred: bool) -> Result<(), String> {
        if deferred {
            return Ok(());
        }
        self.start()
    }

    fn start(&self) -> Result<(), String> {
        let fresh = !self.started.replace(true);
        fresh
            .then(|| NativeCapabilities::within(self.capabilities.clone(), || (self.start)()))
            .unwrap_or(Ok(()))
            .inspect_err(|_| self.dispose())
    }

    fn dispose(&self) {
        let fresh = !self.disposed.replace(true);
        let _ = fresh.then(|| self.stop_started());
        self.capabilities.dispose();
    }

    fn stop_started(&self) {
        if self.started.get() && (self.stop)().is_err() {
            tac_event_failure();
        }
    }
}
