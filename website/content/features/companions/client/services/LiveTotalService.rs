use crate::models::CartTotal;

#[Service]
#[derive(Default)]
struct LiveTotalService {
    seen: i64,
}

impl LiveTotalService {
    #[OnEvent({"replay":true})]
    fn changed(&mut self, changed: CartTotal) { self.seen = changed.total; }
}
