struct NativeApplicationState {
    phase: NativeApplicationPhase,
    page: std::rc::Rc<core::cell::RefCell<NativeServicePage>>,
}

impl NativeApplicationState {
    fn new() -> Self {
        Self {
            phase: NativeApplicationPhase::Idle,
            page: std::rc::Rc::new(core::cell::RefCell::new(NativeServicePage::default())),
        }
    }

    fn begin(&mut self) -> Result<bool, String> {
        match self.phase {
            NativeApplicationPhase::Idle => {
                self.phase = NativeApplicationPhase::Starting;
                Ok(true)
            }
            NativeApplicationPhase::Ready => Ok(false),
            _ => Err(String::from("Native application scope is not available.")),
        }
    }

    fn scope(&self) -> Result<NativeServiceScope, String> {
        self.page().map(|page| NativeServiceScope {
            page,
            owner: String::new(),
            application: true,
        })
    }

    fn ready_scope(&self) -> Result<NativeServiceScope, String> {
        (self.phase == NativeApplicationPhase::Ready)
            .then(|| self.scope())
            .unwrap_or_else(|| Err(String::from("Native application is not running.")))
    }

    fn page(&self) -> Result<std::rc::Rc<core::cell::RefCell<NativeServicePage>>, String> {
        [
            NativeApplicationPhase::Starting,
            NativeApplicationPhase::Ready,
        ]
        .contains(&self.phase)
        .then(|| self.page.clone())
        .ok_or_else(|| String::from("Native application scope is not available."))
    }

    fn require_starting(&self) -> Result<(), String> {
        (self.phase == NativeApplicationPhase::Starting)
            .then_some(())
            .ok_or_else(|| {
                String::from("Native application provider was not initialized by the host.")
            })
    }
}
