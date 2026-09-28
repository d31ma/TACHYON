#[RequestRoute("/status")]
#[Public]
struct StatusController {}
impl StatusController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get_status(&self) -> String {
        String::from("ok")
    }
}
