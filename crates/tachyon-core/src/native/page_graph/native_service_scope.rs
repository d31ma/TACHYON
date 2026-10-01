#[derive(Clone)]
struct NativeServiceScope {
    page: std::rc::Rc<core::cell::RefCell<NativeServicePage>>,
    owner: String,
}

impl NativeServiceScope {
    fn obtain<T: Clone + 'static>(
        &self,
        definition: &NativeServiceDefinition<T>,
    ) -> Result<T, String> {
        self.lookup(definition, false)
    }

    fn obtain_root<T: Clone + 'static>(
        &self,
        definition: &NativeServiceDefinition<T>,
    ) -> Result<T, String> {
        self.lookup(definition, true)
    }

    fn lookup<T: Clone + 'static>(
        &self,
        definition: &NativeServiceDefinition<T>,
        deferred: bool,
    ) -> Result<T, String> {
        let key = self.key(definition);
        let existing = self.page.borrow().node(&key);
        existing.map_or_else(
            || self.construct(key, definition, deferred),
            |node| node.value(),
        )
    }

    fn key<T>(&self, definition: &NativeServiceDefinition<T>) -> (String, String) {
        self.named_key(definition.name, definition.prototype)
    }

    fn named_key(&self, name: &str, prototype: bool) -> (String, String) {
        let owner = prototype.then(|| self.owner.clone()).unwrap_or_default();
        (name.to_owned(), owner)
    }

    fn construct<T: Clone + 'static>(
        &self,
        key: (String, String),
        definition: &NativeServiceDefinition<T>,
        deferred: bool,
    ) -> Result<T, String> {
        self.page.borrow_mut().begin(&key)?;
        let checkpoint = self.page.borrow().order.len();
        let identity = NativePageServices::provider_owner();
        let (value, capabilities) =
            NativeCapabilities::create(&identity, || (definition.create)(self));
        self.page.borrow_mut().end(&key);
        let result = value.and_then(|value| {
            self.remember(key, definition, (value, capabilities.clone(), deferred))
        });
        self.finish(result, capabilities, checkpoint)
    }

    fn remember<T: Clone + 'static>(
        &self,
        key: (String, String),
        definition: &NativeServiceDefinition<T>,
        prepared: (T, std::rc::Rc<NativeCapabilityScope>, bool),
    ) -> Result<T, String> {
        let (value, capabilities, deferred) = prepared;
        let node = NativeServiceNode::new(value.clone(), capabilities, definition);
        node.start_dependency(deferred)?;
        self.page.borrow_mut().insert(key, node);
        Ok(value)
    }

    fn start_root(&self, name: &str, prototype: bool) -> Result<(), String> {
        let node = self
            .page
            .borrow()
            .node(&self.named_key(name, prototype))
            .ok_or_else(|| String::from("Native root provider is missing."))?;
        node.start()
    }

    fn finish<T>(
        &self,
        result: Result<T, String>,
        capabilities: std::rc::Rc<NativeCapabilityScope>,
        checkpoint: usize,
    ) -> Result<T, String> {
        result.inspect_err(|_| {
            capabilities.dispose();
            let nodes = self.page.borrow_mut().take_since(checkpoint);
            NativePageServices::dispose_nodes(nodes);
        })
    }
}
