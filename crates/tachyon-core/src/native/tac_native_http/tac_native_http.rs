#[cfg(not(target_arch = "wasm32"))]
pub struct TacNativeHttp {
    slot: std::sync::Arc<std::sync::Mutex<(Option<Result<TacNativeHttpResponse, &'static str>>, Option<std::task::Waker>)>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl TacNativeHttp {
    const MOST_REQUESTS: usize = 64;
    const CAPACITY: &'static str = "HTTP_CAPACITY";
    const NETWORK: &'static str = "HTTP_NETWORK";
    const DEADLINE: &'static str = "DEADLINE";
    const MISSING: &'static str = "HTTP_MISSING";

    fn pending() -> &'static std::sync::atomic::AtomicUsize {
        static PENDING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        &PENDING
    }

    pub fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
    }

    pub fn start(request: TacNativeHttpRequest) -> Self {
        let exchange = Self {
            slot: std::sync::Arc::new(std::sync::Mutex::new((None, None))),
        };
        let reserved = Self::pending().fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let slot = exchange.slot.clone();
        let started = (reserved < Self::MOST_REQUESTS)
            .then(|| Self::spawn(request, slot.clone()))
            .ok_or(Self::CAPACITY)
            .and_then(|spawned| spawned);
        let _ = started.map_err(|code| Self::settle(&slot, Err(code)));
        exchange
    }

    fn spawn(
        request: TacNativeHttpRequest,
        slot: std::sync::Arc<std::sync::Mutex<(Option<Result<TacNativeHttpResponse, &'static str>>, Option<std::task::Waker>)>>,
    ) -> Result<(), &'static str> {
        std::thread::Builder::new()
            .name(String::from("tachyon-http"))
            .spawn(move || Self::settle(&slot, Self::exchange(&request)))
            .map(|_| ())
            .map_err(|_| Self::NETWORK)
    }

    fn exchange(request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        (request.remaining_ms() > 0)
            .then_some(request)
            .ok_or(Self::DEADLINE)
            .and_then(Self::platform)
    }

    #[cfg(windows)]
    fn platform(request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        TacWinHttp::exchange(request)
    }

    #[cfg(target_os = "linux")]
    fn platform(request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        TacLinuxHttp::exchange(request)
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    fn platform(_request: &TacNativeHttpRequest) -> Result<TacNativeHttpResponse, &'static str> {
        Err(Self::MISSING)
    }

    fn settle(
        slot: &std::sync::Mutex<(Option<Result<TacNativeHttpResponse, &'static str>>, Option<std::task::Waker>)>,
        outcome: Result<TacNativeHttpResponse, &'static str>,
    ) {
        Self::pending().fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        let waker = slot.lock().ok().and_then(|mut slot| {
            slot.0 = Some(outcome);
            slot.1.take()
        });
        let _ = waker.map(std::task::Waker::wake);
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl core::future::Future for TacNativeHttp {
    type Output = Result<TacNativeHttpResponse, &'static str>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        context: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut slot = self
            .slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.1 = Some(context.waker().clone());
        slot.0.take().map_or(core::task::Poll::Pending, core::task::Poll::Ready)
    }
}
