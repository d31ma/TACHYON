#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct CurrentPageEvents;

#[cfg(not(target_arch = "wasm32"))]
impl CurrentPageEvents {
    pub fn publish_event<T: TacModel>(&self, value: T) -> bool {
        ApplicationEventPublisher::event_name::<T>().is_some_and(|name| NativePageEvents::publish(name, &value.write()))
    }
}
