use crate::models::{PlacedOrder, SettledOrder};

#[Service]
#[derive(Default)]
struct OrdersService {
    application_event_publisher: ApplicationEventPublisher,
    log: Vec<String>,
    settled: i64,
    audited: i64,
}

impl OrdersService {
    pub fn place(&mut self, total: i64) {
        let order = PlacedOrder { status: String::from("paid"), total };
        self.application_event_publisher.publish_event(order, false);
    }

    #[OnEvent({"id":"orders-log"})]
    #[Order(-10)]
    fn record(&mut self, order: PlacedOrder) { self.log.push(order.status); }

    #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    fn settle_large_order(&mut self, order: PlacedOrder) -> SettledOrder {
        self.settled += 1;
        SettledOrder { total: order.total }
    }

    #[OnEvent]
    #[InBackground]
    fn audit(&mut self, order: SettledOrder) { self.audited = order.total; }
}
