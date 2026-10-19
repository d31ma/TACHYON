use crate::models::SentRequest;

#[Service]
pub struct OrderService {
    #[Needs]
    orders_client: std::sync::Arc<OrdersClient>,
}

impl OrderService {
    pub async fn replace(&self) -> BehaviorResult<SentRequest> {
        let order = BehaviorValue::object([("item", BehaviorValue::from("hammer"))]);
        self.orders_client.replace(String::from("a-7"), order, String::from("north")).await
    }

    pub async fn change(&self) -> BehaviorResult<SentRequest> {
        let change = BehaviorValue::object([("item", BehaviorValue::from("nails"))]);
        self.orders_client.change(String::from("a-7"), change, String::from("south")).await
    }

    pub async fn remove(&self) -> BehaviorResult<SentRequest> {
        self.orders_client.remove(String::from("a-7"), String::from("duplicate")).await
    }
}
