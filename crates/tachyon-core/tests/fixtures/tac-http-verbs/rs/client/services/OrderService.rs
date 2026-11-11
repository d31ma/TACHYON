use crate::models::SentRequest;

#[Service]
pub struct OrderService {
    #[Needs]
    orders_client: std::sync::Arc<OrdersClient>,
}

impl OrderService {
    pub async fn create(&self) -> BehaviorResult<SentRequest> {
        let order = BehaviorValue::object([("item", BehaviorValue::from("anvil"))]);
        self.orders_client.create(order, String::from("west")).await
    }

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

    pub async fn claim(&self) -> BehaviorResult<SentRequest> {
        let claim = BehaviorValue::object([("item", BehaviorValue::from("saw"))]);
        self.orders_client.claim(String::from("a-7"), claim).await
    }
}
