
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    #[link_name = "tac_host_epoch_ms"]
    fn tac_web_epoch_ms() -> f64;
    #[link_name = "tac_host_monotonic_ms"]
    fn tac_web_monotonic_ms() -> f64;
    fn tac_host_web(request: *const u8, length: usize) -> *mut u8;
}

pub struct TacWebHost;

impl TacWebHost {
    const NO_PAGE: &'static str = "This browser API needs the page, and this service runs without one.";

    #[cfg(target_arch = "wasm32")]
    pub fn epoch_ms() -> f64 {
        unsafe { tac_web_epoch_ms() }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn epoch_ms() -> f64 {
        let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        elapsed.unwrap_or_default().as_secs_f64() * 1000.0
    }

    #[cfg(target_arch = "wasm32")]
    pub fn monotonic_ms() -> f64 {
        unsafe { tac_web_monotonic_ms() }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn monotonic_ms() -> f64 {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }

    pub fn call(api: &str, call: &str, arguments: &str) -> Result<TacValue, String> {
        let request = format!(
            "{{\"api\":{},\"call\":{},\"args\":[{arguments}]}}",
            tac_quote(api),
            tac_quote(call)
        );
        let answer = Self::ask(&request)?;
        let entries = TacScanner::new(&answer).object();
        Self::value_or_error(&entries)
    }

    fn value_or_error(entries: &[(String, TacValue)]) -> Result<TacValue, String> {
        match tac_entry_text(entries, "error") {
            Some(message) => Err(message.clone()),
            None => Ok(tac_entry(entries, "value").cloned().unwrap_or_default()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn ask(request: &str) -> Result<String, String> {
        let answer = unsafe { tac_host_web(request.as_ptr(), request.len()) };
        Self::owned_answer(answer)
    }

    #[cfg(target_arch = "wasm32")]
    fn owned_answer(answer: *mut u8) -> Result<String, String> {
        if answer.is_null() {
            return Err(String::from(Self::NO_PAGE));
        }
        let bytes = unsafe { std::ffi::CStr::from_ptr(answer.cast()) }.to_bytes_with_nul();
        let length = bytes.len();
        let text = String::from_utf8_lossy(&bytes[..length - 1]).into_owned();
        drop(unsafe { Vec::from_raw_parts(answer, 0, length) });
        Ok(text)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ask(request: &str) -> Result<String, String> {
        let _ = request;
        Err(String::from(Self::NO_PAGE))
    }
}
