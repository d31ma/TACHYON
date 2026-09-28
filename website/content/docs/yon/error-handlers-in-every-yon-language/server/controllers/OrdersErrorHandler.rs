use crate::models::Problem;

#[ErrorHandler]
struct OrdersErrorHandler {}

impl OrdersErrorHandler {
    fn new() -> Self {
        Self {}
    }

    #[HandlesError]
    #[ResponseStatus(404)]
    fn missing(&self, error: &OrderNotFound) -> Problem {
        Problem { code: String::from("order-not-found") }
    }
}
