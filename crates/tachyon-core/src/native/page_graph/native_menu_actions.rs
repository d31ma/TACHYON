struct NativeMenuActions;

impl NativeMenuActions {
    const NAME_LIMIT: usize = 64;

    fn run(
        id: *const core::ffi::c_char,
        dispatch: impl FnOnce(&str) -> Result<(), String>,
    ) -> bool {
        Self::text(id).is_some_and(|id| Self::claimed(&id, dispatch))
    }

    fn ready(event: *const core::ffi::c_char) -> bool {
        let idle = !tac_handling().with(core::cell::Cell::get);
        let listens = Self::text(event).is_some_and(|event| NativePageEvents::listens(&event));
        [idle, listens] == [true; 2]
    }

    fn claimed(id: &str, dispatch: impl FnOnce(&str) -> Result<(), String>) -> bool {
        let idle = !tac_handling().with(|handling| handling.replace(true));
        idle.then(|| Self::dispatched(id, dispatch))
            .unwrap_or_default()
    }

    fn dispatched(id: &str, dispatch: impl FnOnce(&str) -> Result<(), String>) -> bool {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch(id)))
            .unwrap_or_else(|_| Err(String::from("Native menu action panicked.")));
        tac_handling().with(|handling| handling.set(false));
        result.is_ok()
    }

    fn text(pointer: *const core::ffi::c_char) -> Option<String> {
        (!pointer.is_null())
            .then(|| unsafe { core::ffi::CStr::from_ptr(pointer) }.to_bytes())
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .filter(|text| (1..=Self::NAME_LIMIT).contains(&text.len()))
            .map(str::to_owned)
    }
}
