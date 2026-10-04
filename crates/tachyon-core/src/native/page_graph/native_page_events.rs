struct NativePageEvents;

impl NativePageEvents {
    const VALUE_LIMIT: usize = 65_536;

    fn delivering() -> &'static std::thread::LocalKey<core::cell::Cell<bool>> {
        thread_local! { static VALUE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) }; }
        &VALUE
    }

    fn listens(name: &str) -> bool {
        !Self::recipients(name).is_empty()
    }

    fn publish(name: &str, value: &TacValue) -> bool {
        let valid = tac_publishable(value, 0) && tac_write(value).len() <= Self::VALUE_LIMIT;
        if !valid || Self::delivering().with(|delivering| delivering.replace(true)) {
            tac_event_failure();
            return false;
        }
        let recipients = Self::recipients(name);
        let previous = tac_executing().with(|slot| slot.borrow().clone());
        let delivered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recipients
                .iter()
                .for_each(|owner| Self::deliver(owner, name, value));
        }));
        tac_executing().with(|slot| *slot.borrow_mut() = previous);
        Self::delivering().with(|delivering| delivering.set(false));
        delivered.is_ok() && !recipients.is_empty()
    }

    fn deliver(owner: &std::rc::Rc<TacActivation>, name: &str, value: &TacValue) {
        tac_set_executing(owner);
        tac_notify_listeners(owner, name, value);
        tac_drain(owner);
    }

    fn recipients(name: &str) -> Vec<std::rc::Rc<TacActivation>> {
        let owners =
            tac_active_owners().with(|slot| slot.borrow().values().cloned().collect::<Vec<_>>());
        let mut seen = std::collections::BTreeSet::new();
        owners
            .into_iter()
            .filter(|owner| !owner.disposed.get() && Self::listening(owner, name))
            .filter(|owner| seen.insert(Self::identity(owner)))
            .collect()
    }

    fn listening(owner: &TacActivation, name: &str) -> bool {
        owner.listeners.iter().any(|index| {
            matches!(owner.members.get(*index), Some((_, TacMember::Listener(listener))) if listener.events.contains(&name))
        })
    }

    fn identity(owner: &TacActivation) -> (String, String) {
        match NativePageServices::prototype_root(&owner.route, &owner.owner) {
            true => (owner.route.clone(), owner.owner.clone()),
            false => (owner.route.clone(), owner.service.clone()),
        }
    }
}
