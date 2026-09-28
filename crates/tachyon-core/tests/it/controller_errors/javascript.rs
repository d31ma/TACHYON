use super::{Commands, Files, Http, Project, Server};
use std::path::Path;

impl Project {
    pub(crate) const JAVASCRIPT: Self = Self {
        extension: "js",
        program: "node",
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
                "/**\n * @typedef {Object} Problem\n * @property {string} code\n */\nexport {}\n",
            ),
            (
                "services/OrdersService",
                r"@Service
class OrdersService {
  find() { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
",
            ),
            (
                "controllers/OrdersController",
                r#"@RequestRoute("/orders")
@Public
@NeedsAll()
class OrdersController {
  /** @type {string} */
  static _PRIVATE_MESSAGE = 'PRIVATE_ERROR_SENTINEL';
  ordersService;
  /** @returns {Problem} */
  @GetRoute
  getOrder() { throw new OrderNotFound(OrdersController._PRIVATE_MESSAGE); }
  /** @returns {Problem} */
  @GetRoute("/gone")
  getGone() { throw new OrderGone(OrdersController._PRIVATE_MESSAGE); }
  /** @returns {Problem} */
  @GetRoute("/service")
  getService() { return this.ordersService.find(); }
  /** @returns {Problem} */
  @GetRoute("/unknown")
  getUnknown() { throw new TypeError(OrdersController._PRIVATE_MESSAGE); }
  /** @returns {Problem} */
  @GetRoute("/broken")
  getBroken() { throw new OrderBroken(OrdersController._PRIVATE_MESSAGE); }
  /** @returns {Problem} */
  @GetRoute("/invalid")
  getInvalid() { throw new OrderInvalid(OrdersController._PRIVATE_MESSAGE); }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"@RequestRoute("/status")
@Public
class StatusController {
  /** @returns {Problem} */
  @GetRoute
  getStatus() { throw new OrderNotFound('PRIVATE_ERROR_SENTINEL'); }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r"@ErrorHandler
class OrdersErrorHandler {
  /** @type {string} */
  static _PRIVATE_MESSAGE = 'PRIVATE_ERROR_SENTINEL';
  /**
   * @param {OrderNotFound} error
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(404)
  missing(error) { return { code: 'missing' }; }
  /**
   * @param {OrderGone} error
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(410)
  gone(error) { return { code: 'gone' }; }
  /**
   * @param {OrderBroken} error
   * @param {Request} request
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(503)
  broken(error, request) { throw new TypeError(OrdersErrorHandler._PRIVATE_MESSAGE); }
  /**
   * @param {OrderInvalid} error
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(422)
  invalid(error) { return { wrong: OrdersErrorHandler._PRIVATE_MESSAGE }; }
}
",
            ),
        ],
        checks_invalid_model: true,
    };
}

struct MiddlewareProject;

impl MiddlewareProject {
    const MIDDLEWARE: &str = r"export class Middleware {
  static after(request) {
    return new Response(null, { status: 204, headers: { 'x-after-status': request.headers.get('x-tachyon-status') ?? '' } })
  }
}
";
    const CONTROLLER: &str = r#"@RequestRoute("/orders")
@Public
class OrdersController {
  /** @type {string} */
  static _PRIVATE_MESSAGE = 'PRIVATE_ERROR_SENTINEL';
  /** @returns {Problem} */
  @GetRoute
  getOrder() { throw new OrderNotFound(OrdersController._PRIVATE_MESSAGE); }
  /** @returns {Problem} */
  @GetRoute("/later")
  async getLater() {
    await Promise.resolve();
    throw new OrderNotFound(OrdersController._PRIVATE_MESSAGE);
  }
}
"#;
    const HANDLER: &str = r"@ErrorHandler
class OrdersErrorHandler {
  /**
   * @param {OrderNotFound} error
   * @returns {Problem}
   */
  @HandlesError
  @ResponseStatus(404)
  missing(error) { return { code: 'missing' }; }
}
";

    fn write(root: &Path) {
        Files::write(root, "middleware.js", Self::MIDDLEWARE);
        Files::write(
            root,
            "server/models/Problem.js",
            "/**\n * @typedef {Object} Problem\n * @property {string} code\n */\nexport {}\n",
        );
        Files::write(
            root,
            "server/errors/OrderNotFound.js",
            "class OrderNotFound extends Error {}\n",
        );
        Files::write(
            root,
            "server/controllers/OrdersController.js",
            Self::CONTROLLER,
        );
        Files::write(
            root,
            "server/controllers/OrdersErrorHandler.js",
            Self::HANDLER,
        );
    }

    async fn responses(paths: &[&str]) -> Option<Vec<String>> {
        if !Commands::available("node", "js").await {
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        Self::write(root.path());
        Some(Server::responses(root.path(), paths).await)
    }

    fn check_missing(response: &str) {
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
        assert_eq!(
            Http::json_body(response),
            serde_json::json!({ "code": "missing" }),
            "{response}"
        );
        assert!(!response.contains("PRIVATE_ERROR_SENTINEL"), "{response}");
    }
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::JAVASCRIPT.verify().await;
}

#[tokio::test]
async fn middleware_after_sees_the_handled_answer() {
    let Some(responses) = MiddlewareProject::responses(&["/orders"]).await else {
        return;
    };
    let response = &responses[0];
    MiddlewareProject::check_missing(response);
    let header = response.to_ascii_lowercase();
    assert!(header.contains("x-after-status: 404"), "{response}");
}

#[tokio::test]
async fn an_async_controller_rejection_is_handled() {
    let Some(responses) = MiddlewareProject::responses(&["/orders/later"]).await else {
        return;
    };
    MiddlewareProject::check_missing(&responses[0]);
}
