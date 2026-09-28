use super::{Commands, Project};

impl Project {
    pub(crate) const TYPESCRIPT: Self = Self {
        extension: "ts",
        program: "bun",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound extends Error {}\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone extends OrderNotFound {}\n",
            ),
            ("errors/OrderBroken", "class OrderBroken extends Error {}\n"),
            (
                "errors/OrderInvalid",
                "class OrderInvalid extends Error {}\n",
            ),
            (
                "models/Problem",
                "export interface Problem { code: string }\n",
            ),
            (
                "services/OrdersService",
                r"import type { Problem } from '/models/Problem.ts';
@Service
class OrdersService {
  find(): Problem { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
",
            ),
            (
                "controllers/OrdersController",
                r#"import type { Problem } from '/models/Problem.ts';
@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
  private static readonly PRIVATE_MESSAGE: string = 'PRIVATE_ERROR_SENTINEL';
  readonly ordersService!: OrdersService;
  @GetRoute
  getOrder(): Problem { throw new OrderNotFound(OrdersController.PRIVATE_MESSAGE); }
  @GetRoute("/gone")
  getGone(): Problem { throw new OrderGone(OrdersController.PRIVATE_MESSAGE); }
  @GetRoute("/service")
  getService(): Problem { return this.ordersService.find(); }
  @GetRoute("/unknown")
  getUnknown(): Problem { throw new TypeError(OrdersController.PRIVATE_MESSAGE); }
  @GetRoute("/broken")
  getBroken(): Problem { throw new OrderBroken(OrdersController.PRIVATE_MESSAGE); }
  @GetRoute("/invalid")
  getInvalid(): Problem { throw new OrderInvalid(OrdersController.PRIVATE_MESSAGE); }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"import type { Problem } from '/models/Problem.ts';
@RequestRoute("/status")
@Public
class StatusController {
  @GetRoute
  getStatus(): Problem { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r"import type { Problem } from '/models/Problem.ts';
@ErrorHandler
class OrdersErrorHandler {
  private static readonly PRIVATE_MESSAGE: string = 'PRIVATE_ERROR_SENTINEL';
  @HandlesError
  @ResponseStatus(404)
  missing(error: OrderNotFound): Problem { return { code: 'missing' }; }
  @HandlesError
  @ResponseStatus(410)
  gone(error: OrderGone): Problem { return { code: 'gone' }; }
  @HandlesError
  @ResponseStatus(503)
  broken(error: OrderBroken, request: Request): Problem { throw new TypeError(OrdersErrorHandler.PRIVATE_MESSAGE); }
  @HandlesError
  @ResponseStatus(422)
  invalid(error: OrderInvalid): Problem { return { wrong: OrdersErrorHandler.PRIVATE_MESSAGE } as any; }
}
",
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    if std::env::var("YON_JAVASCRIPT_RUNTIME").as_deref() != Ok("bun") {
        let executable = std::env::current_exe().expect("test executable");
        let arguments = [
            "--exact",
            "controller_errors::typescript::error_handlers_answer_every_scenario",
            "--nocapture",
        ];
        let environment = [("YON_JAVASCRIPT_RUNTIME", "bun")];
        assert!(Commands::success(&executable, &arguments, &environment, 120).await);
        return;
    }
    Project::TYPESCRIPT.verify().await;
}
