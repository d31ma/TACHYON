use crate::models::Greeting;

#[RequestRoute("/api/greeting")]
#[Public]
struct GreetingController {}
impl GreetingController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_greeting(&self) -> Greeting {
        Greeting { message: String::from("Hello from Yon") }
    }
}
