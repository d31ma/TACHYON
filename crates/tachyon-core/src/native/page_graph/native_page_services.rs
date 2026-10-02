struct NativePageServices;

impl NativePageServices {
    const OWNER_PREFIX: &'static str = "00000000-0000-4000-a000-";

    fn pages() -> &'static std::thread::LocalKey<
        core::cell::RefCell<
            std::collections::BTreeMap<String, std::rc::Rc<core::cell::RefCell<NativeServicePage>>>,
        >,
    > {
        thread_local! { static VALUE: core::cell::RefCell<std::collections::BTreeMap<String, std::rc::Rc<core::cell::RefCell<NativeServicePage>>>> = const { core::cell::RefCell::new(std::collections::BTreeMap::new()) }; }
        &VALUE
    }

    fn current() -> &'static std::thread::LocalKey<
        core::cell::RefCell<Option<std::rc::Rc<NativeServiceActivation>>>,
    > {
        thread_local! { static VALUE: core::cell::RefCell<Option<std::rc::Rc<NativeServiceActivation>>> = const { core::cell::RefCell::new(None) }; }
        &VALUE
    }

    fn activation() -> Option<std::rc::Rc<NativeServiceActivation>> {
        Self::current().with(|current| current.borrow().clone())
    }

    fn pending() -> &'static std::thread::LocalKey<
        core::cell::RefCell<
            std::collections::BTreeMap<String, std::rc::Rc<NativeServiceActivation>>,
        >,
    > {
        thread_local! { static VALUE: core::cell::RefCell<std::collections::BTreeMap<String, std::rc::Rc<NativeServiceActivation>>> = const { core::cell::RefCell::new(std::collections::BTreeMap::new()) }; }
        &VALUE
    }

    fn build<T>(route: &str, owner: &str, create: impl FnOnce() -> T) -> Result<T, String> {
        let scope = Self::scope(route, owner)?;
        let checkpoint = scope.page.borrow().order.len();
        let activation = std::rc::Rc::new(NativeServiceActivation {
            scope,
            failure: core::cell::RefCell::new(None),
            checkpoint,
        });
        let previous = Self::current().with(|current| current.replace(Some(activation.clone())));
        let result = create();
        Self::current().with(|current| current.replace(previous));
        Self::finish_build(activation, result)
    }

    fn finish_build<T>(
        activation: std::rc::Rc<NativeServiceActivation>,
        result: T,
    ) -> Result<T, String> {
        if let Some(failure) = activation.failure.take() {
            let nodes = activation
                .scope
                .page
                .borrow_mut()
                .take_since(activation.checkpoint);
            Self::dispose_nodes(nodes);
            return Err(failure);
        }
        Self::pending().with(|pending| {
            pending
                .borrow_mut()
                .insert(activation.scope.owner.clone(), activation)
        });
        Ok(result)
    }

    fn commit(owner: &str) {
        Self::pending().with(|pending| pending.borrow_mut().remove(owner));
    }

    fn rollback(owner: &str) {
        let pending = Self::pending().with(|pending| pending.borrow_mut().remove(owner));
        let _ = pending.map(|activation| {
            let nodes = activation
                .scope
                .page
                .borrow_mut()
                .take_since(activation.checkpoint);
            Self::dispose_nodes(nodes);
        });
    }

    fn scope(route: &str, owner: &str) -> Result<NativeServiceScope, String> {
        if owner.to_ascii_lowercase().starts_with(Self::OWNER_PREFIX) {
            return Err(String::from("Reserved native provider owner."));
        }
        let page = Self::pages().with(|pages| {
            pages
                .borrow_mut()
                .entry(route.to_owned())
                .or_default()
                .clone()
        });
        Ok(NativeServiceScope {
            page,
            owner: owner.to_owned(),
            application: false,
        })
    }

    fn provider_owner() -> String {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let identity = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("{}{:012x}", Self::OWNER_PREFIX, identity)
    }

    fn dispose_owner(owner: &str) {
        let pages =
            Self::pages().with(|pages| pages.borrow().values().cloned().collect::<Vec<_>>());
        for page in pages {
            let nodes = page.borrow_mut().take_owner(owner);
            Self::dispose_nodes(nodes);
        }
    }

    fn dispose_all() {
        Self::pending().with(|pending| pending.borrow_mut().clear());
        let pages = Self::pages().with(|pages| std::mem::take(&mut *pages.borrow_mut()));
        for page in pages.into_values() {
            let nodes = page.borrow_mut().take_since(0);
            Self::dispose_nodes(nodes);
        }
    }

    fn dispose_nodes(nodes: Vec<std::rc::Rc<NativeServiceNode>>) {
        for node in nodes {
            node.dispose();
        }
    }
}
