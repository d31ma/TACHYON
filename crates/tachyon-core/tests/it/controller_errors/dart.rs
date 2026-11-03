use super::Project;

impl Project {
    pub(crate) const DART: Self = Self {
        extension: "dart",
        program: "dart",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound implements Exception {\n  final String message;\n  OrderNotFound(this.message);\n}\n",
            ),
            (
                "errors/OrderGone",
                "class OrderGone extends OrderNotFound {\n  OrderGone(super.message);\n}\n",
            ),
            (
                "errors/OrderBroken",
                "class OrderBroken extends Error {\n  final String message;\n  OrderBroken(this.message);\n}\n",
            ),
            (
                "errors/OrderInvalid",
                "class OrderInvalid implements Exception {\n  final String message;\n  OrderInvalid(this.message);\n}\n",
            ),
            (
                "models/Problem",
                "class Problem {\n  final String code;\n  const Problem(this.code);\n}\n",
            ),
            (
                "services/OrdersService",
                r"import 'models/Problem.dart';
@Service
class OrdersService {
  Future<Problem> find() async {
    throw OrderNotFound('PRIVATE_ERROR_SENTINEL');
  }
}
",
            ),
            (
                "controllers/OrdersController",
                r#"import 'models/Problem.dart';
@RequestRoute("/orders")
@Public
class OrdersController {
  static const String _privateMessage = 'PRIVATE_ERROR_SENTINEL';
  @Needs
  final OrdersService orders;
  @GetRoute
  Problem getOrder() {
    throw OrderNotFound(_privateMessage);
  }
  @GetRoute("/gone")
  Future<Problem> getGone() async {
    throw OrderGone(_privateMessage);
  }
  @GetRoute("/service")
  Future<Problem> getService() {
    return orders.find();
  }
  @GetRoute("/unknown")
  Problem getUnknown() {
    throw StateError(_privateMessage);
  }
  @GetRoute("/broken")
  Problem getBroken() {
    throw OrderBroken(_privateMessage);
  }
  @GetRoute("/invalid")
  Problem getInvalid() {
    throw OrderInvalid(_privateMessage);
  }
}
"#,
            ),
            (
                "controllers/StatusController",
                r#"import 'models/Problem.dart';
@RequestRoute("/status")
@Public
class StatusController {
  @GetRoute
  Problem getStatus() {
    throw OrderNotFound('PRIVATE_ERROR_SENTINEL');
  }
}
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r"import 'models/Problem.dart';
@ErrorHandler
class OrdersErrorHandler {
  @HandlesError
  @ResponseStatus(404)
  Problem missing(OrderNotFound error) {
    return const Problem('missing');
  }
  @HandlesError
  @ResponseStatus(410)
  Problem gone(OrderGone error) {
    return const Problem('gone');
  }
  @HandlesError
  @ResponseStatus(503)
  Problem broken(OrderBroken error, Object request) {
    throw StateError('PRIVATE_ERROR_SENTINEL');
  }
  @HandlesError
  @ResponseStatus(422)
  Problem invalid(OrderInvalid error) {
    return const Problem('invalid');
  }
}
",
            ),
        ],
        checks_invalid_model: false,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::DART.verify().await;
}
