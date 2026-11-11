use crate::models::Problem;

#[Service]
pub struct OrderService {
    #[Needs]
    order_client: std::sync::Arc<OrderClient>,
    pub status: i64,
    pub title: String,
    pub mistakes: i64,
}

impl OrderService {
    pub async fn send(&mut self) -> BehaviorResult<()> {
        let order = BehaviorValue::object([("name", BehaviorValue::from("")), ("level", BehaviorValue::from(12))]);
        let error = self.order_client.send(order).await.err().unwrap_or_else(|| BehaviorError::new("none", "none"));
        let problem: Problem = error.body_as().unwrap_or_default();
        self.status = i64::from(error.status);
        self.title = problem.title;
        self.mistakes = problem.errors.len() as i64;
        Ok(())
    }
}
