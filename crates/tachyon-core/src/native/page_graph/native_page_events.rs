struct NativePageEvents;

impl NativePageEvents {
    const VALUE_LIMIT: usize = 65_536;

    fn delivering() -> &'static std::thread::LocalKey<core::cell::Cell<bool>> {
        thread_local! { static VALUE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) }; }
        &VALUE
    }

    fn listens(name: &str) -> bool {
        !Self::listeners(name).is_empty()
    }

    fn publish(name: &str, value: &TacValue) -> bool {
        let claimed = Self::publishable(value)
            .then(Self::claim)
            .unwrap_or_default();
        claimed
            .then(|| Self::deliver_all(name, value))
            .unwrap_or_else(Self::refused)
    }

    fn publishable(value: &TacValue) -> bool {
        [
            tac_publishable(value, 0),
            tac_write(value).len() <= Self::VALUE_LIMIT,
        ] == [true; 2]
    }

    fn claim() -> bool {
        !Self::delivering().with(|delivering| delivering.replace(true))
    }

    fn refused() -> bool {
        tac_event_failure();
        false
    }

    fn deliver_all(name: &str, value: &TacValue) -> bool {
        let listeners = Self::listeners(name);
        let recipients = Self::distinct(&listeners);
        let previous = tac_executing().with(|slot| slot.borrow().clone());
        let delivered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recipients
                .iter()
                .for_each(|owner| Self::deliver(owner, name, value));
        }));
        tac_executing().with(|slot| *slot.borrow_mut() = previous);
        Self::delivering().with(|delivering| delivering.set(false));
        listeners.iter().for_each(|owner| Self::announce(owner));
        [delivered.is_ok(), !recipients.is_empty()] == [true; 2]
    }

    fn deliver(owner: &std::rc::Rc<TacActivation>, name: &str, value: &TacValue) {
        tac_set_executing(owner);
        tac_notify_listeners(owner, name, value);
        tac_drain(owner);
    }

    fn announce(owner: &TacActivation) {
        let emit = tac_emit_slot().lock().ok().and_then(|installed| *installed);
        let payload = std::ffi::CString::new(Self::changed(owner)).ok();
        if let (Some(emit), Some(payload)) = (emit, payload) {
            emit(payload.as_ptr());
        }
    }

    fn changed(owner: &TacActivation) -> String {
        format!(
            "{{\"route\":{},\"service\":{},\"owner\":{},\"changed\":true}}",
            tac_quote(&owner.route),
            tac_quote(&owner.service),
            tac_quote(&owner.owner)
        )
    }

    fn listeners(name: &str) -> Vec<std::rc::Rc<TacActivation>> {
        let owners =
            tac_active_owners().with(|slot| slot.borrow().values().cloned().collect::<Vec<_>>());
        owners
            .into_iter()
            .filter(|owner| Self::live_listener(owner, name))
            .collect()
    }

    fn distinct(listeners: &[std::rc::Rc<TacActivation>]) -> Vec<std::rc::Rc<TacActivation>> {
        let mut seen = std::collections::BTreeSet::new();
        listeners
            .iter()
            .filter(|owner| seen.insert(Self::identity(owner)))
            .cloned()
            .collect()
    }

    fn live_listener(owner: &TacActivation, name: &str) -> bool {
        [!owner.disposed.get(), Self::listening(owner, name)] == [true; 2]
    }

    fn listening(owner: &TacActivation, name: &str) -> bool {
        owner
            .listeners
            .iter()
            .filter_map(|index| owner.members.get(*index))
            .any(|(_, member)| Self::hears(member, name))
    }

    fn hears(member: &TacMember, name: &str) -> bool {
        matches!(member, TacMember::Listener(listener) if listener.events.contains(&name))
    }

    fn identity(owner: &TacActivation) -> (String, String) {
        let member = if NativePageServices::prototype_root(&owner.route, &owner.owner) {
            owner.owner.clone()
        } else {
            owner.service.clone()
        };
        (owner.route.clone(), member)
    }
}
