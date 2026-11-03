pub struct BehaviorTasks;

impl BehaviorTasks {
    const WAITED: &'static str = "The test case waited for the browser. A Tac test unit has no browser HTTP transport; give the client an HTTP adapter in the TestSetup.";

    pub fn finish<T>(future: impl core::future::Future<Output = T>) -> T {
        let mut future = core::pin::pin!(future);
        let mut context = core::task::Context::from_waker(core::task::Waker::noop());
        Self::ready(future.as_mut().poll(&mut context))
    }

    fn ready<T>(poll: core::task::Poll<T>) -> T {
        match poll {
            core::task::Poll::Ready(value) => value,
            core::task::Poll::Pending => panic!("{}", Self::WAITED),
        }
    }
}
