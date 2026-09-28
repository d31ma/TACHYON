use super::Project;

impl Project {
    pub(crate) const PHP: Self = Self {
        extension: "php",
        program: "php",
        files: &[
            (
                "errors/OrderNotFound",
                "<?php\nclass OrderNotFound extends RuntimeException {}\n",
            ),
            (
                "errors/OrderGone",
                "<?php\nclass OrderGone extends OrderNotFound {}\n",
            ),
            (
                "errors/OrderBroken",
                "<?php\nclass OrderBroken extends RuntimeException {}\n",
            ),
            (
                "errors/OrderInvalid",
                "<?php\nclass OrderInvalid extends RuntimeException {}\n",
            ),
            (
                "models/Problem",
                "<?php\nclass Problem\n{\n    public function __construct(public string $code) {}\n}\n",
            ),
            (
                "services/OrdersService",
                r"<?php
use Models\Problem;

#[Service]
class OrdersService {
    public function find(): Problem { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
",
            ),
            (
                "controllers/OrdersController",
                r#"<?php
use Models\Problem;

#[RequestRoute("/orders")]
#[Public]
#[Needs(OrdersService::class)]
class OrdersController {
    private const PRIVATE_MESSAGE = 'PRIVATE_ERROR_SENTINEL';
    public function __construct(public OrdersService $orders) {}
    #[GetRoute]
    public function getOrder(): Problem { throw new OrderNotFound(self::PRIVATE_MESSAGE); }
    #[GetRoute("/gone")]
    public function getGone(): Problem { throw new OrderGone(self::PRIVATE_MESSAGE); }
    #[GetRoute("/service")]
    public function getService(): Problem { return $this->orders->find(); }
    #[GetRoute("/unknown")]
    public function getUnknown(): Problem { throw new LogicException(self::PRIVATE_MESSAGE); }
    #[GetRoute("/broken")]
    public function getBroken(): Problem { throw new OrderBroken(self::PRIVATE_MESSAGE); }
    #[GetRoute("/invalid")]
    public function getInvalid(): Problem { throw new OrderInvalid(self::PRIVATE_MESSAGE); }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"<?php
use Models\Problem;

#[RequestRoute("/status")]
#[Public]
class StatusController {
    #[GetRoute]
    public function getStatus(): Problem { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r"<?php
use Models\Problem;

#[ErrorHandler]
class OrdersErrorHandler {
    private const PRIVATE_MESSAGE = 'PRIVATE_ERROR_SENTINEL';
    #[HandlesError]
    #[ResponseStatus(404)]
    public function missing(OrderNotFound $error): Problem { return new Problem('missing'); }
    #[HandlesError]
    #[ResponseStatus(410)]
    public function gone(OrderGone $error): Problem { return new Problem('gone'); }
    #[HandlesError]
    #[ResponseStatus(503)]
    public function broken(OrderBroken $error): Problem { throw new LogicException(self::PRIVATE_MESSAGE); }
    #[HandlesError]
    #[ResponseStatus(422)]
    public function invalid(OrderInvalid $error): Problem {
        $problem = new Problem('invalid');
        $problem->wrong = self::PRIVATE_MESSAGE;
        return $problem;
    }
}
",
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::PHP.verify().await;
}
