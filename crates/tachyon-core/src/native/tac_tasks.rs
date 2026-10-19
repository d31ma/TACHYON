pub type TacTask =
    core::pin::Pin<Box<dyn core::future::Future<Output = Result<TacValue, TacTaskError>>>>;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "tachyon")]
unsafe extern "C" {
    fn tac_host_task_done(task: u32, answer: *const u8, length: usize);
}

#[derive(Default)]
struct TacTasks {
    last: u32,
    running: std::collections::BTreeMap<u32, (String, TacTask)>,
    ready: std::collections::VecDeque<u32>,
}

impl TacTasks {
    const MOST_TASKS: usize = 256;
    const CANCELLED: &'static str =
        "{\"error\":\"The service was removed before its task finished.\",\"code\":\"CANCELLED\"}";
    const TOO_MANY: &'static str =
        "{\"error\":\"The page runs too many service tasks at once.\",\"code\":\"CAPACITY\"}";
    #[cfg(not(target_arch = "wasm32"))]
    const NO_HOST: &'static str = "{\"error\":\"This method waits for the browser, and this service runs without one.\",\"code\":\"HTTP_MISSING\"}";

    fn table() -> &'static std::thread::LocalKey<core::cell::RefCell<TacTasks>> {
        thread_local! {
            static TASKS: core::cell::RefCell<TacTasks> = core::cell::RefCell::new(TacTasks::default());
        }
        &TASKS
    }

    fn begin(owner: &str, task: TacTask) -> String {
        Self::reserve().map_or_else(
            || String::from(Self::TOO_MANY),
            |id| Self::first_poll(id, owner, task),
        )
    }

    fn first_poll(id: u32, owner: &str, mut task: TacTask) -> String {
        match Self::poll(id, &mut task) {
            core::task::Poll::Ready(result) => Self::answer(result),
            core::task::Poll::Pending => Self::park(id, owner, task),
        }
    }

    fn reserve() -> Option<u32> {
        Self::table().with(|table| {
            let mut table = table.borrow_mut();
            if table.running.len() >= Self::MOST_TASKS {
                return None;
            }
            table.last = table.last.wrapping_add(1).max(1);
            Some(table.last)
        })
    }

    fn poll(id: u32, task: &mut TacTask) -> core::task::Poll<Result<TacValue, TacTaskError>> {
        let waker = std::task::Waker::from(std::sync::Arc::new(TacTaskWaker { task: id }));
        task.as_mut()
            .poll(&mut core::task::Context::from_waker(&waker))
    }

    #[cfg(target_arch = "wasm32")]
    fn park(id: u32, owner: &str, task: TacTask) -> String {
        Self::put(id, owner.to_owned(), task);
        format!("{{\"task\":{id}}}")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn park(id: u32, owner: &str, task: TacTask) -> String {
        TacTaskDispatch::installed().map_or_else(
            || String::from(Self::NO_HOST),
            |_| {
                Self::put(id, owner.to_owned(), task);
                format!("{{\"task\":{id}}}")
            },
        )
    }

    fn put(id: u32, owner: String, task: TacTask) {
        Self::table().with(|table| table.borrow_mut().running.insert(id, (owner, task)));
    }

    #[cfg(target_arch = "wasm32")]
    fn wake(id: u32) {
        Self::table().with(|table| table.borrow_mut().ready.push_back(id));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn wake(id: u32) {
        TacTaskDispatch::wake(id);
    }

    #[cfg(target_arch = "wasm32")]
    fn next_ready() -> Option<u32> {
        Self::table().with(|table| table.borrow_mut().ready.pop_front())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn next_ready() -> Option<u32> {
        TacTaskDispatch::next()
    }

    fn run_ready() {
        tac_handling().with(|handling| handling.set(true));
        Self::drain_ready();
        tac_handling().with(|handling| handling.set(false));
    }

    fn drain_ready() {
        while let Some(id) = Self::next_ready() {
            Self::resume(id);
        }
    }

    fn resume(id: u32) {
        let taken = Self::table().with(|table| table.borrow_mut().running.remove(&id));
        let _ = taken.map(|(owner, task)| Self::continue_task(id, owner, task));
    }

    fn continue_task(id: u32, owner: String, task: TacTask) {
        let activation = tac_active_owner(&owner);
        let _ = activation.as_ref().map(tac_set_executing);
        Self::poll_again(id, owner, task);
        let _ = activation.as_ref().map(|activation| tac_drain(activation));
        tac_executing().with(|slot| slot.borrow_mut().take());
    }

    fn poll_again(id: u32, owner: String, mut task: TacTask) {
        match Self::poll(id, &mut task) {
            core::task::Poll::Ready(result) => Self::finish(id, &owner, &Self::answer(result)),
            core::task::Poll::Pending => Self::put(id, owner, task),
        }
    }

    fn cancel_owner(owner: &str) {
        let removed = Self::table().with(|table| {
            let mut table = table.borrow_mut();
            let ids: Vec<u32> = table
                .running
                .iter()
                .filter(|(_, (task_owner, _))| task_owner == owner)
                .map(|(id, _)| *id)
                .collect();
            ids.into_iter()
                .filter_map(|id| table.running.remove(&id).map(|entry| (id, entry)))
                .collect::<Vec<_>>()
        });
        Self::cancel(removed);
    }

    fn cancel_all() {
        let removed = Self::table().with(|table| {
            let running = core::mem::take(&mut table.borrow_mut().running);
            running.into_iter().collect::<Vec<_>>()
        });
        Self::cancel(removed);
    }

    fn cancel(removed: Vec<(u32, (String, TacTask))>) {
        for (id, (owner, task)) in removed {
            drop(task);
            Self::finish(id, &owner, Self::CANCELLED);
        }
    }

    fn answer(result: Result<TacValue, TacTaskError>) -> String {
        result.map_or_else(
            |error| {
                format!(
                    "{{\"error\":{},\"code\":{}}}",
                    tac_quote(error.message),
                    tac_quote(error.code)
                )
            },
            |value| format!("{{\"value\":{}}}", tac_write(&value)),
        )
    }

    #[cfg(target_arch = "wasm32")]
    fn finish(id: u32, _owner: &str, answer: &str) {
        unsafe { tac_host_task_done(id, answer.as_ptr(), answer.len()) };
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn finish(id: u32, owner: &str, answer: &str) {
        TacTaskDispatch::finish(id, owner, answer);
    }
}
