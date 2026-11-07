use super::Project;

impl Project {
    pub(crate) const JAVA: Self = Self {
        extension: "java",
        program: "java",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound extends RuntimeException {\n    OrderNotFound(String message) { super(message); }\n}\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone extends OrderNotFound {\n    OrderGone(String message) { super(message); }\n}\n",
            ),
            (
                "errors/OrderBroken",
                "class OrderBroken extends Exception {\n    OrderBroken(String message) { super(message); }\n}\n",
            ),
            (
                "errors/OrderInvalid",
                "class OrderInvalid extends RuntimeException {\n    OrderInvalid(String message) { super(message); }\n}\n",
            ),
            ("models/Problem", "record Problem(String code) {}\n"),
            (
                "services/OrdersService",
                "import models.Problem;\n@Service\nclass OrdersService {\n    Problem find() { throw new OrderNotFound(\"PRIVATE_ERROR_SENTINEL\"); }\n}\n",
            ),
            (
                "controllers/OrdersController",
                r#"import models.Problem;
@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
    private static final String PRIVATE_MESSAGE = "PRIVATE_ERROR_SENTINEL";
    OrdersService orders;
    @GetRoute
    Problem getOrder() { throw new OrderNotFound(PRIVATE_MESSAGE); }
    @GetRoute("/gone")
    Problem getGone() { throw new OrderGone(PRIVATE_MESSAGE); }
    @GetRoute("/service")
    Problem getService() { return orders.find(); }
    @GetRoute("/unknown")
    Problem getUnknown() { throw new IllegalArgumentException(PRIVATE_MESSAGE); }
    @GetRoute("/broken")
    Problem getBroken() throws OrderBroken { throw new OrderBroken(PRIVATE_MESSAGE); }
    @GetRoute("/invalid")
    Problem getInvalid() { throw new OrderInvalid(PRIVATE_MESSAGE); }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"import models.Problem;
@RequestRoute("/status")
@Public
class StatusController {
    @GetRoute
    Problem getStatus() { throw new OrderNotFound("PRIVATE_ERROR_SENTINEL"); }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r#"import models.Problem;
@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    @ResponseStatus(404)
    Problem missing(OrderNotFound error) { return new Problem("missing"); }
    @HandlesError
    @ResponseStatus(410)
    Problem gone(OrderGone error) { return new Problem("gone"); }
    @HandlesError
    @ResponseStatus(503)
    Problem broken(OrderBroken error, com.sun.net.httpserver.HttpExchange request) throws OrderBroken { throw new OrderBroken("PRIVATE_ERROR_SENTINEL"); }
    @HandlesError
    @ResponseStatus(422)
    Problem invalid(OrderInvalid error) { return new Problem(null); }
}
"#,
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::JAVA.verify().await;
}
