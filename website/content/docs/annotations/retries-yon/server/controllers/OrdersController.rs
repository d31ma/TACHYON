use crate::models::OrderReport;
use std::sync::Arc;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {
    #[Needs]
    orders: Arc<OrdersService>,
}
impl OrdersController {
    #[GetRoute]
    fn get_orders(&self) -> OrderReport {
        let id = BehaviorValue::from("A-1");
        let submitted = self.orders.submit(id.clone()).expect("submitted");
        let report = self.orders.report(id).expect("report");
        OrderReport { submitted: Self::json(submitted), report: Self::json(report) }
    }

    fn json(value: BehaviorValue) -> Json {
        Json::parse(&value.to_json(65536).expect("serialize")).expect("parse")
    }
}
