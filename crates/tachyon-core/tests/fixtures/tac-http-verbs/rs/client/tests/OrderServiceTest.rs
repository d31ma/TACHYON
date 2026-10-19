#[Test(OrdersTestSetup, {"partition":"tac-http-verbs"})]
struct OrderServiceTest {
    behaviors: std::cell::RefCell<Option<BehaviorContext>>,
    #[Fake]
    orders_client: std::cell::RefCell<Option<std::sync::Arc<OrdersClient>>>,
}
impl OrderServiceTest {
    fn new() -> Self {
        Self {
            behaviors: std::cell::RefCell::new(None),
            orders_client: std::cell::RefCell::new(None),
        }
    }

    async fn testPutSendsThePathTheHeaderAndTheBody(&self) {
        let service: OrderService = self.construct();
        let sent = service.replace().await;
        Assert::equals(None, sent.as_ref().err().map(|error| error.code));
        let sent = sent.unwrap_or_default();
        Assert::equals("PUT", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("north", sent.header);
        Assert::equals("hammer", sent.item);
    }

    async fn testPatchSendsThePathTheHeaderAndTheBody(&self) {
        let service: OrderService = self.construct();
        let sent = service.change().await;
        Assert::equals(None, sent.as_ref().err().map(|error| error.code));
        let sent = sent.unwrap_or_default();
        Assert::equals("PATCH", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("south", sent.header);
        Assert::equals("nails", sent.item);
    }

    async fn testDeleteSendsThePathAndTheHeaderWithoutABody(&self) {
        let service: OrderService = self.construct();
        let sent = service.remove().await;
        Assert::equals(None, sent.as_ref().err().map(|error| error.code));
        let sent = sent.unwrap_or_default();
        Assert::equals("DELETE", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("duplicate", sent.header);
        Assert::equals("none", sent.item);
    }
}
