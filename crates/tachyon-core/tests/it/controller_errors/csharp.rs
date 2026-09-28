use super::Project;

impl Project {
    pub(crate) const CSHARP: Self = Self {
        extension: "cs",
        program: "dotnet",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound : System.Exception {\n    public OrderNotFound(string message) : base(message) {}\n}\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone : OrderNotFound {\n    public OrderGone(string message) : base(message) {}\n}\n",
            ),
            (
                "errors/OrderBroken",
                "class OrderBroken : System.Exception {\n    public OrderBroken(string message) : base(message) {}\n}\n",
            ),
            (
                "errors/OrderInvalid",
                "class OrderInvalid : System.Exception {\n    public OrderInvalid(string message) : base(message) {}\n}\n",
            ),
            ("models/Problem", "public record Problem(string Code);\n"),
            (
                "services/OrdersService",
                r#"using Models;
[Service]
class OrdersService {
    public Problem Find() { throw new OrderNotFound("PRIVATE_ERROR_SENTINEL"); }
}
"#,
            ),
            (
                "controllers/OrdersController",
                r#"using Models;
[RequestRoute("/orders")]
[Public]
[Needs(typeof(OrdersService))]
class OrdersController {
    private const string PrivateMessage = "PRIVATE_ERROR_SENTINEL";
    readonly OrdersService orders;
    public OrdersController(OrdersService orders) { this.orders = orders; }
    [GetRoute]
    public Problem GetOrder() { throw new OrderNotFound(PrivateMessage); }
    [GetRoute("/gone")]
    public Problem GetGone() { throw new OrderGone(PrivateMessage); }
    [GetRoute("/service")]
    public Problem GetService() { return orders.Find(); }
    [GetRoute("/unknown")]
    public Problem GetUnknown() { throw new System.InvalidOperationException(PrivateMessage); }
    [GetRoute("/broken")]
    public Problem GetBroken() { throw new OrderBroken(PrivateMessage); }
    [GetRoute("/invalid")]
    public Problem GetInvalid() { throw new OrderInvalid(PrivateMessage); }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"using Models;
[RequestRoute("/status")]
[Public]
class StatusController {
    [GetRoute]
    public Problem GetStatus() { throw new OrderNotFound("PRIVATE_ERROR_SENTINEL"); }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r#"using Models;
[ErrorHandler]
class OrdersErrorHandler {
    [HandlesError]
    [ResponseStatus(404)]
    public Problem Missing(OrderNotFound error) { return new Problem("missing"); }
    [HandlesError]
    [ResponseStatus(410)]
    public Problem Gone(OrderGone error) { return new Problem("gone"); }
    [HandlesError]
    [ResponseStatus(503)]
    public Problem Broken(OrderBroken error, HttpRequestMessage request) { throw new System.InvalidOperationException("PRIVATE_ERROR_SENTINEL"); }
    [HandlesError]
    [ResponseStatus(422)]
    public Problem Invalid(OrderInvalid error) { return new Problem(null); }
}
"#,
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::CSHARP.verify().await;
}
