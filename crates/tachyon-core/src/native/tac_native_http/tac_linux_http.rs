#[cfg(target_os = "linux")]
type TacLinuxHttpSender = extern "C" fn(
    u64,
    *const core::ffi::c_char,
    *const core::ffi::c_char,
    *const core::ffi::c_char,
    *const u8,
    usize,
    u64,
    usize,
);

#[cfg(target_os = "linux")]
type TacLinuxHttpOutcome = Result<TacNativeHttpResponse, &'static str>;

#[cfg(target_os = "linux")]
pub struct TacLinuxHttp;

#[cfg(target_os = "linux")]
impl TacLinuxHttp {
    const MISSING: &'static str = "HTTP_MISSING";
    const NETWORK: &'static str = "HTTP_NETWORK";
    const DEADLINE: &'static str = "DEADLINE";
    const REQUEST: &'static str = "HTTP_REQUEST";
    const HOST_CODES: [&'static str; 4] = ["DEADLINE", "HTTP_LIMIT", "HTTP_REQUEST", "HTTP_NETWORK"];
    const GRACE_MS: u64 = 1_000;

    fn sender() -> &'static std::sync::Mutex<Option<TacLinuxHttpSender>> {
        static SENDER: std::sync::Mutex<Option<TacLinuxHttpSender>> = std::sync::Mutex::new(None);
        &SENDER
    }

    fn waiting(
    ) -> &'static std::sync::Mutex<std::collections::BTreeMap<u64, std::sync::mpsc::SyncSender<TacLinuxHttpOutcome>>>
    {
        static WAITING: std::sync::Mutex<
            std::collections::BTreeMap<u64, std::sync::mpsc::SyncSender<TacLinuxHttpOutcome>>,
        > = std::sync::Mutex::new(std::collections::BTreeMap::new());
        &WAITING
    }

    fn tickets() -> &'static std::sync::atomic::AtomicU64 {
        static TICKETS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        &TICKETS
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn tac_native_set_http_sender(sender: Option<TacLinuxHttpSender>) {
        let _ = Self::sender().lock().map(|mut installed| *installed = sender);
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn tac_native_http_answer(
        ticket: u64,
        status: u32,
        body: *const u8,
        length: usize,
        error: *const core::ffi::c_char,
    ) {
        let read = (!body.is_null()).then(|| unsafe { std::slice::from_raw_parts(body, length) });
        let named = (!error.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(error) });
        let outcome = Self::outcome(status, read.unwrap_or_default(), named);
        let waiter = Self::waiting().lock().ok().and_then(|mut waiting| waiting.remove(&ticket));
        let _ = waiter.map(|waiter| waiter.send(outcome));
    }

    fn outcome(status: u32, body: &[u8], error: Option<&std::ffi::CStr>) -> TacLinuxHttpOutcome {
        error.map_or_else(|| Self::response(status, body), |named| Err(Self::code(named)))
    }

    fn response(status: u32, body: &[u8]) -> TacLinuxHttpOutcome {
        u16::try_from(status)
            .map(|status| TacNativeHttpResponse { status, body: body.to_vec() })
            .map_err(|_| Self::NETWORK)
    }

    fn code(named: &std::ffi::CStr) -> &'static str {
        let text = named.to_bytes();
        Self::HOST_CODES
            .into_iter()
            .find(|code| code.as_bytes() == text)
            .unwrap_or(Self::NETWORK)
    }

    pub fn exchange(request: &TacNativeHttpRequest) -> TacLinuxHttpOutcome {
        let sender = Self::sender().lock().ok().and_then(|installed| *installed).ok_or(Self::MISSING)?;
        let method = std::ffi::CString::new(request.method).map_err(|_| Self::REQUEST)?;
        let url = std::ffi::CString::new(format!("{}{}", request.origin, request.target)).map_err(|_| Self::REQUEST)?;
        let headers = std::ffi::CString::new(Self::header_lines(&request.headers)).map_err(|_| Self::REQUEST)?;
        let body = request.body.as_deref().map(str::as_bytes);
        let ticket = Self::tickets().fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (answer, answered) = std::sync::mpsc::sync_channel(1);
        let _ = Self::waiting().lock().map(|mut waiting| waiting.insert(ticket, answer));
        sender(
            ticket,
            method.as_ptr(),
            url.as_ptr(),
            headers.as_ptr(),
            body.map_or(core::ptr::null(), <[u8]>::as_ptr),
            body.map_or(0, <[u8]>::len),
            request.remaining_ms(),
            request.limit,
        );
        let wait = std::time::Duration::from_millis(request.remaining_ms().saturating_add(Self::GRACE_MS));
        let outcome = answered.recv_timeout(wait).unwrap_or(Err(Self::DEADLINE));
        let _ = Self::waiting().lock().map(|mut waiting| waiting.remove(&ticket));
        outcome
    }

    fn header_lines(headers: &[(String, String)]) -> String {
        headers
            .iter()
            .map(|(name, value)| format!("{name}\n{value}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
