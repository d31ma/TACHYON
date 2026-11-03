#[cfg(not(target_arch = "wasm32"))]
struct BehaviorNativeHttp;

#[cfg(not(target_arch = "wasm32"))]
impl BehaviorNativeHttp {
    async fn send(text: String) -> BehaviorResult<String> {
        drop(text);
        Err(behavior_error(
            "HTTP_MISSING",
            "The default HTTP transport runs only in the browser. Give this client an HTTP adapter in its Setup.",
        ))
    }
}
