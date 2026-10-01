pub struct NativeCapabilities;

impl NativeCapabilities {
    #[unsafe(no_mangle)]
    pub extern "C" fn tac_native_set_capability_adapter(
        call: Option<NativeCapabilityCall>,
        free: Option<NativeCapabilityFree>,
    ) {
        Self::set_adapter(NativeCapabilityAdapter::registered(call, free));
    }

    fn adapter() -> &'static std::sync::Mutex<NativeCapabilityAdapter> {
        static VALUE: std::sync::Mutex<NativeCapabilityAdapter> =
            std::sync::Mutex::new(NativeCapabilityAdapter::Unsupported);
        &VALUE
    }

    fn constructing() -> &'static std::thread::LocalKey<
        core::cell::RefCell<Option<std::rc::Rc<NativeCapabilityScope>>>,
    > {
        thread_local! { static VALUE: core::cell::RefCell<Option<std::rc::Rc<NativeCapabilityScope>>> = const { core::cell::RefCell::new(None) }; }
        &VALUE
    }

    pub fn register(call: fn(&str, &str) -> String) {
        Self::set_adapter(NativeCapabilityAdapter::Local(call));
    }

    fn set_adapter(adapter: NativeCapabilityAdapter) {
        if let Ok(mut current) = Self::adapter().lock() {
            *current = adapter;
        }
    }

    fn scope() -> std::rc::Rc<NativeCapabilityScope> {
        Self::constructing()
            .with(|value| value.borrow().clone())
            .unwrap_or_else(|| {
                std::rc::Rc::new(NativeCapabilityScope::new(
                    "",
                    NativeCapabilityAdapter::Unsupported,
                ))
            })
    }

    fn create<T>(owner: &str, work: impl FnOnce() -> T) -> (T, std::rc::Rc<NativeCapabilityScope>) {
        let adapter = Self::adapter()
            .lock()
            .map_or(NativeCapabilityAdapter::Unsupported, |value| *value);
        let scope = std::rc::Rc::new(NativeCapabilityScope::new(owner, adapter));
        let previous = Self::constructing().with(|value| value.replace(Some(scope.clone())));
        let result = work();
        Self::constructing().with(|value| value.replace(previous));
        (result, scope)
    }
}
