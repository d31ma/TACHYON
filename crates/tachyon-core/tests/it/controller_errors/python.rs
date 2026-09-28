use super::{Commands, Project, Server};
use std::path::Path;

impl Project {
    pub(crate) const PYTHON: Self = Self {
        extension: "py",
        program: "python3",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound(Exception):\n    pass\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone(OrderNotFound):\n    pass\n",
            ),
            (
                "errors/OrderBroken",
                "class OrderBroken(Exception):\n    pass\n",
            ),
            (
                "errors/OrderInvalid",
                "class OrderInvalid(Exception):\n    pass\n",
            ),
            (
                "models/Problem",
                "from dataclasses import dataclass\n@dataclass\nclass Problem:\n    code: str\n",
            ),
            (
                "services/OrdersService",
                "from models import Problem\n@Service\nclass OrdersService:\n    def find(self) -> Problem:\n        raise OrderNotFound('PRIVATE_ERROR_SENTINEL')\n",
            ),
            (
                "controllers/OrdersController",
                r#"from models import Problem
@RequestRoute("/orders")
@Public
@needs_all()
class OrdersController:
    PRIVATE_MESSAGE: str = 'PRIVATE_ERROR_SENTINEL'
    orders: OrdersService
    @GetRoute
    def get_order(self) -> Problem:
        raise OrderNotFound(self.PRIVATE_MESSAGE)
    @GetRoute("/gone")
    def get_gone(self) -> Problem:
        raise OrderGone(self.PRIVATE_MESSAGE)
    @GetRoute("/service")
    def get_service(self) -> Problem:
        return self.orders.find()
    @GetRoute("/unknown")
    def get_unknown(self) -> Problem:
        raise ValueError(self.PRIVATE_MESSAGE)
    @GetRoute("/broken")
    def get_broken(self) -> Problem:
        raise OrderBroken(self.PRIVATE_MESSAGE)
    @GetRoute("/invalid")
    def get_invalid(self) -> Problem:
        raise OrderInvalid(self.PRIVATE_MESSAGE)
"#,
            ),
            (
                "controllers/StatusController",
                r#"from models import Problem
@RequestRoute("/status")
@Public
class StatusController:
    @GetRoute
    def get_status(self) -> Problem:
        raise OrderNotFound('PRIVATE_ERROR_SENTINEL')
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r"from models import Problem
@ErrorHandler
class OrdersErrorHandler:
    PRIVATE_MESSAGE: str = 'PRIVATE_ERROR_SENTINEL'
    @HandlesError
    @ResponseStatus(404)
    def missing(self, error: OrderNotFound) -> Problem:
        return Problem('missing')
    @HandlesError
    @ResponseStatus(410)
    def gone(self, error: OrderGone) -> Problem:
        return Problem('gone')
    @HandlesError
    @ResponseStatus(503)
    def broken(self, error: OrderBroken, request) -> Problem:
        raise ValueError(self.PRIVATE_MESSAGE)
    @HandlesError
    @ResponseStatus(422)
    def invalid(self, error: OrderInvalid) -> Problem:
        return {'wrong': self.PRIVATE_MESSAGE}
",
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::PYTHON.verify().await;
}

struct PythonProjects;

impl PythonProjects {
    const NOT_AN_ERROR: (&'static str, &'static str) =
        ("errors/NotAnError", "class NotAnError:\n    pass\n");
    const NOT_AN_ERROR_METHOD: &'static str = "    @HandlesError\n    @ResponseStatus(400)\n    def odd(self, error: NotAnError) -> Problem:\n        return Problem('odd')\n";

    fn changed(replacements: &[(&str, &str)]) -> Project {
        let mut files = Vec::new();
        for (name, source) in Project::PYTHON.files {
            let mut source = (*source).to_owned();
            for (from, to) in replacements {
                source = source.replace(from, to);
            }
            let source: &'static str = Box::leak(source.into_boxed_str());
            files.push((*name, source));
        }
        Project {
            files: Box::leak(files.into_boxed_slice()),
            ..Project::PYTHON
        }
    }

    fn write_with_non_exception_handler(root: &Path) {
        Project::PYTHON.write(root);
        let (name, source) = Self::NOT_AN_ERROR;
        super::Files::write(root, &format!("server/{name}.py"), source);
        let handler = root.join("server/controllers/OrdersErrorHandler.py");
        let mut source = std::fs::read_to_string(&handler).expect("handler");
        source.push_str(Self::NOT_AN_ERROR_METHOD);
        std::fs::write(handler, source).expect("handler");
    }
}

#[tokio::test]
async fn asynchronous_controllers_services_and_handlers_answer_every_scenario() {
    let replacements = [
        ("    def ", "    async def "),
        (
            "return self.orders.find()",
            "return await self.orders.find()",
        ),
    ];
    PythonProjects::changed(&replacements).verify().await;
}

#[tokio::test]
async fn a_handler_for_a_type_that_is_not_an_exception_fails_closed() {
    if !Commands::available("python3", "py").await {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    PythonProjects::write_with_non_exception_handler(root.path());
    let paths = ["/orders", "/orders/gone", "/status", "/orders"];
    let responses = Server::responses(root.path(), &paths).await;
    for (path, response) in paths.iter().zip(&responses) {
        Project::PYTHON.check_safe_failure(path, response);
    }
}
