use super::Project;

impl Project {
    pub(crate) const KOTLIN: Self = Self {
        extension: "kt",
        program: "kotlinc",
        files: &[
            (
                "errors/OrderNotFound",
                "open class OrderNotFound(message: String) : RuntimeException(message)\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone(message: String) : OrderNotFound(message)\n",
            ),
            (
                "errors/OrderBroken",
                "class OrderBroken(message: String) : RuntimeException(message)\n",
            ),
            ("models/Problem", "data class Problem(val code: String)\n"),
            (
                "services/OrdersService",
                "import models.Problem\n@Service\nclass OrdersService {\n    fun find(): Problem { throw OrderNotFound(\"PRIVATE_ERROR_SENTINEL\") }\n}\n",
            ),
            (
                "controllers/OrdersController",
                r#"import models.Problem
@RequestRoute("/orders")
@Public
@NeedsAll
class OrdersController {
    private companion object { const val PRIVATE_MESSAGE = "PRIVATE_ERROR_SENTINEL" }
    val orders: OrdersService
    @GetRoute
    fun getOrder(): Problem { throw OrderNotFound(PRIVATE_MESSAGE) }
    @GetRoute("/gone")
    fun getGone(): Problem { throw OrderGone(PRIVATE_MESSAGE) }
    @GetRoute("/service")
    fun getService(): Problem { return orders.find() }
    @GetRoute("/unknown")
    fun getUnknown(): Problem { throw IllegalArgumentException(PRIVATE_MESSAGE) }
    @GetRoute("/broken")
    fun getBroken(): Problem { throw OrderBroken(PRIVATE_MESSAGE) }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"import models.Problem
@RequestRoute("/status")
@Public
class StatusController {
    @GetRoute
    fun getStatus(): Problem { throw OrderNotFound("PRIVATE_ERROR_SENTINEL") }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r#"import models.Problem
@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    @ResponseStatus(404)
    fun missing(error: OrderNotFound): Problem { return Problem("missing") }
    @HandlesError
    @ResponseStatus(410)
    fun gone(error: OrderGone): Problem { return Problem("gone") }
    @HandlesError
    @ResponseStatus(503)
    fun broken(error: OrderBroken, request: com.sun.net.httpserver.HttpExchange): Problem { throw IllegalStateException("PRIVATE_ERROR_SENTINEL") }
}
"#,
            ),
        ],
        checks_invalid_model: false,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::KOTLIN.verify().await;
}
