#[Service]
struct OrderService;

impl OrderService {
    fn list() -> Vec<&'static str> {
        vec!["anvil"]
    }
}

impl OrderService {
    fn new() -> Self { Self }
}
