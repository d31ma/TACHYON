#[cfg(not(target_arch = "wasm32"))]
pub struct TacTaskDispatch;

#[cfg(not(target_arch = "wasm32"))]
impl TacTaskDispatch {
    const SIGNAL_LIMIT: usize = 65_536;
    const TOO_LARGE: &'static str =
        "{\"error\":\"The task result is larger than 64 KiB.\",\"code\":\"HTTP_LIMIT\"}";

    fn hook() -> &'static std::sync::Mutex<Option<extern "C" fn()>> {
        static HOOK: std::sync::Mutex<Option<extern "C" fn()>> = std::sync::Mutex::new(None);
        &HOOK
    }

    fn ready() -> &'static std::sync::Mutex<std::collections::VecDeque<u32>> {
        static READY: std::sync::Mutex<std::collections::VecDeque<u32>> =
            std::sync::Mutex::new(std::collections::VecDeque::new());
        &READY
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn tac_native_set_dispatch(dispatch: Option<extern "C" fn()>) {
        let _ = Self::hook().lock().map(|mut hook| *hook = dispatch);
    }

    fn installed() -> Option<extern "C" fn()> {
        Self::hook().lock().ok().and_then(|hook| *hook)
    }

    fn wake(task: u32) {
        let _ = Self::ready().lock().map(|mut ready| ready.push_back(task));
        let _ = Self::installed().map(|dispatch| dispatch());
    }

    fn next() -> Option<u32> {
        Self::ready().lock().ok().and_then(|mut ready| ready.pop_front())
    }

    fn finish(task: u32, owner: &str, answer: &str) {
        let emit = tac_emit_slot().lock().ok().and_then(|installed| *installed);
        let signal = tac_active_owner(owner)
            .map(|activation| Self::bounded(task, &activation, answer))
            .and_then(|payload| std::ffi::CString::new(payload).ok());
        let _ = emit
            .zip(signal)
            .map(|(emit, signal)| emit(signal.as_ptr()));
    }

    fn bounded(task: u32, activation: &TacActivation, answer: &str) -> String {
        let payload = Self::payload(task, activation, answer);
        (payload.len() <= Self::SIGNAL_LIMIT)
            .then_some(payload)
            .unwrap_or_else(|| Self::payload(task, activation, Self::TOO_LARGE))
    }

    fn payload(task: u32, activation: &TacActivation, answer: &str) -> String {
        format!(
            "{{\"route\":{},\"service\":{},\"owner\":{},\"task\":{task},\"answer\":{}}}",
            tac_quote(&activation.route),
            tac_quote(&activation.service),
            tac_quote(&activation.owner),
            tac_quote(answer)
        )
    }
}
