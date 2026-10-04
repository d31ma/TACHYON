struct NativeApplicationServices;

impl NativeApplicationServices {
    fn host_thread() -> &'static std::sync::OnceLock<std::thread::ThreadId> {
        static VALUE: std::sync::OnceLock<std::thread::ThreadId> = std::sync::OnceLock::new();
        &VALUE
    }

    fn claim_thread() -> Result<(), String> {
        Self::host_thread().get_or_init(|| std::thread::current().id());
        Self::require_thread()
    }

    fn require_thread() -> Result<(), String> {
        (Self::host_thread().get() == Some(&std::thread::current().id()))
            .then_some(())
            .ok_or_else(|| {
                String::from("Native application access requires its original host thread.")
            })
    }

    fn state() -> &'static std::thread::LocalKey<core::cell::RefCell<NativeApplicationState>> {
        thread_local! { static VALUE: core::cell::RefCell<NativeApplicationState> = core::cell::RefCell::new(NativeApplicationState::new()); }
        &VALUE
    }

    fn start(create: impl FnOnce(&NativeServiceScope) -> Result<(), String>) -> Result<(), String> {
        Self::claim_thread()?;
        let fresh = Self::state().with(|state| state.borrow_mut().begin())?;
        fresh.then(|| Self::build(create)).transpose().map(|_| ())
    }

    fn build(create: impl FnOnce(&NativeServiceScope) -> Result<(), String>) -> Result<(), String> {
        let scope = Self::state().with(|state| state.borrow().scope())?;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| create(&scope)))
            .unwrap_or_else(|_| Err(String::from("Native application startup panicked.")));
        Self::finish(result)
    }

    fn finish(result: Result<(), String>) -> Result<(), String> {
        result
            .map(|()| Self::ready())
            .inspect_err(|_| Self::shutdown())
    }

    fn ready() {
        Self::state().with(|state| state.borrow_mut().phase = NativeApplicationPhase::Ready);
    }

    fn page() -> Result<std::rc::Rc<core::cell::RefCell<NativeServicePage>>, String> {
        Self::require_thread()?;
        Self::state().with(|state| state.borrow().page())
    }

    fn obtain<T: Clone + 'static>(definition: &NativeServiceDefinition<T>) -> Result<T, String> {
        Self::require_thread()?;
        let scope = Self::state().with(|state| state.borrow().ready_scope())?;
        scope.obtain(definition)
    }

    fn require_starting() -> Result<(), String> {
        Self::require_thread()?;
        Self::state().with(|state| state.borrow().require_starting())
    }

    fn dispose_nodes(nodes: Vec<std::rc::Rc<NativeServiceNode>>) {
        for node in nodes {
            Self::dispose_node(&node);
        }
    }

    fn dispose_node(node: &NativeServiceNode) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| node.dispose()));
        node.capabilities.dispose();
    }

    fn host_shutdown() {
        let _ = Self::require_thread().map(|()| {
            tac_dispose_all();
            Self::retire();
        });
    }

    fn shutdown() {
        let _ = Self::require_thread().map(|()| Self::retire());
    }

    fn retire() {
        let page = Self::state().with(|state| {
            let mut state = state.borrow_mut();
            state.phase = NativeApplicationPhase::Retired;
            state.page.clone()
        });
        let nodes = page.borrow_mut().take_since(0);
        Self::dispose_nodes(nodes);
    }
}
