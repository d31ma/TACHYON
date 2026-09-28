#[Service]
struct OrderService;

impl OrderService {
    fn list() -> Vec<&'static str> {
        OrderStore::all()
    }
}

impl OrderService {
    fn new() -> Self { Self }
}
