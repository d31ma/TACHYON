struct NativeMenuActions;

impl NativeMenuActions {
    const NAME_LIMIT: usize = 64;

    fn run(
        id: *const core::ffi::c_char,
        dispatch: impl FnOnce(&str) -> Result<(), String>,
    ) -> bool {
        let Some(id) = Self::text(id) else {
            return false;
        };
        if tac_handling().with(|handling| handling.replace(true)) {
            return false;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch(&id)))
            .unwrap_or_else(|_| Err(String::from("Native menu action panicked.")));
        tac_handling().with(|handling| handling.set(false));
        result.is_ok()
    }

    fn ready(event: *const core::ffi::c_char) -> bool {
        let idle = !tac_handling().with(core::cell::Cell::get);
        idle && Self::text(event).is_some_and(|event| NativePageEvents::listens(&event))
    }

    fn text(pointer: *const core::ffi::c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        let bytes = unsafe { core::ffi::CStr::from_ptr(pointer) }.to_bytes();
        let text = core::str::from_utf8(bytes).ok()?;
        (!text.is_empty() && text.len() <= Self::NAME_LIMIT).then(|| text.to_owned())
    }
}
