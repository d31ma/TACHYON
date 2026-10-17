#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    fn tac_host_http_start(ticket: u32, request: *const u8, length: usize);
    fn tac_host_http_cancel(ticket: u32);
    fn tac_host_socket_start(
        ticket: u32,
        owner: *const u8,
        owner_length: usize,
        request: *const u8,
        length: usize,
    );
}

#[cfg(target_arch = "wasm32")]
pub struct TacHttpTicket {
    ticket: u32,
    settled: bool,
}

#[cfg(target_arch = "wasm32")]
impl TacHttpTicket {
    const LOST: &'static str = "{\"error\":{\"code\":\"HTTP_MISSING\"}}";

    fn slots() -> &'static std::thread::LocalKey<
        core::cell::RefCell<
            std::collections::BTreeMap<u32, (Option<String>, Option<std::task::Waker>)>,
        >,
    > {
        thread_local! {
            static SLOTS: core::cell::RefCell<std::collections::BTreeMap<u32, (Option<String>, Option<std::task::Waker>)>> =
                const { core::cell::RefCell::new(std::collections::BTreeMap::new()) };
        }
        &SLOTS
    }

    fn last() -> &'static std::thread::LocalKey<core::cell::Cell<u32>> {
        thread_local! {
            static LAST: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
        }
        &LAST
    }

    pub fn start(request: &str) -> Self {
        let ticket = Self::reserve();
        unsafe { tac_host_http_start(ticket, request.as_ptr(), request.len()) };
        Self {
            ticket,
            settled: false,
        }
    }

    pub fn socket(owner: &str, request: &str) -> Self {
        let ticket = Self::reserve();
        unsafe {
            tac_host_socket_start(
                ticket,
                owner.as_ptr(),
                owner.len(),
                request.as_ptr(),
                request.len(),
            )
        };
        Self {
            ticket,
            settled: false,
        }
    }

    fn reserve() -> u32 {
        let ticket = Self::last().with(|last| {
            let next = last.get().wrapping_add(1).max(1);
            last.set(next);
            next
        });
        Self::slots().with(|slots| slots.borrow_mut().insert(ticket, (None, None)));
        ticket
    }

    fn settle(ticket: u32, answer: String) {
        let waker = Self::slots().with(|slots| {
            let mut slots = slots.borrow_mut();
            let slot = slots.get_mut(&ticket)?;
            slot.0 = Some(answer);
            slot.1.take()
        });
        let _ = waker.map(std::task::Waker::wake);
    }

    fn take(
        slots: &mut std::collections::BTreeMap<u32, (Option<String>, Option<std::task::Waker>)>,
        ticket: u32,
        waker: &std::task::Waker,
    ) -> Option<String> {
        match slots.remove(&ticket) {
            Some((Some(answer), _)) => Some(answer),
            Some((None, _)) => {
                slots.insert(ticket, (None, Some(waker.clone())));
                None
            }
            None => Some(String::from(Self::LOST)),
        }
    }

    fn cancel_when(pending: bool, ticket: u32) {
        if pending {
            Self::slots().with(|slots| slots.borrow_mut().remove(&ticket));
            unsafe { tac_host_http_cancel(ticket) };
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn tac_http_settle(ticket: u32, answer: *const u8, length: usize) {
        let bytes = unsafe { core::slice::from_raw_parts(answer, length) };
        Self::settle(ticket, String::from_utf8_lossy(bytes).into_owned());
        TacTasks::run_ready();
    }
}

#[cfg(target_arch = "wasm32")]
impl core::future::Future for TacHttpTicket {
    type Output = String;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        context: &mut core::task::Context<'_>,
    ) -> core::task::Poll<String> {
        let ticket = self.ticket;
        let answer = Self::slots()
            .with(|slots| Self::take(&mut slots.borrow_mut(), ticket, context.waker()));
        self.settled = answer.is_some();
        answer.map_or(core::task::Poll::Pending, core::task::Poll::Ready)
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for TacHttpTicket {
    fn drop(&mut self) {
        Self::cancel_when(!self.settled, self.ticket);
    }
}
