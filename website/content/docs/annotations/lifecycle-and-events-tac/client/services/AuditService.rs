use crate::models::{PlacedOrder, SettledOrder};

#[derive(Default)]
#[Service]
struct AuditService {
    records: i64,
    settled: i64,
    audited: i64,
}
impl AuditService {
    #[OnEvent({"id":"audit-log","replay":true})]
    #[Order(-10)]
    fn record(&mut self, _order: PlacedOrder) { self.records += 1; }

    #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    #[Order(10)]
    fn settle(&mut self, order: PlacedOrder) -> SettledOrder {
        SettledOrder { total: order.total }
    }

    #[OnEvent]
    fn save_settled(&mut self, order: SettledOrder) { self.settled = order.total; }

    #[OnEvent]
    #[InBackground]
    fn audit(&mut self, order: SettledOrder) { self.audited = order.total; }
}
