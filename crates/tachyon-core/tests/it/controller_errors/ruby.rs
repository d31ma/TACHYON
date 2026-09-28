use super::Project;

impl Project {
    pub(crate) const RUBY: Self = Self {
        extension: "rb",
        program: "ruby",
        files: &[
            (
                "errors/OrderNotFound",
                "class OrderNotFound < StandardError\nend\n",
            ),
            ("errors/OrderGone", "class OrderGone < OrderNotFound\nend\n"),
            (
                "errors/OrderBroken",
                "class OrderBroken < StandardError\nend\n",
            ),
            (
                "errors/OrderInvalid",
                "class OrderInvalid < StandardError\nend\n",
            ),
            (
                "models/Problem",
                "# @!attribute [r] code\n#   @return [String]\nProblem = Data.define(:code)\n",
            ),
            (
                "services/OrdersService",
                "#[Service]\nclass OrdersService\n  # @return [Problem]\n  def find\n    raise OrderNotFound, \"PRIVATE_ERROR_SENTINEL\"\n  end\nend\n",
            ),
            (
                "controllers/OrdersController",
                r#"#[RequestRoute("/orders")]
#[Public]
#[Needs(OrdersService)]
class OrdersController
  PRIVATE_MESSAGE = "PRIVATE_ERROR_SENTINEL"
  def initialize(orders)
    @orders = orders
  end

  # @return [Problem]
  #[GetRoute]
  def get_order
    raise OrderNotFound, PRIVATE_MESSAGE
  end

  # @return [Problem]
  #[GetRoute("/gone")]
  def get_gone
    raise OrderGone, PRIVATE_MESSAGE
  end

  # @return [Problem]
  #[GetRoute("/service")]
  def get_service
    @orders.find
  end

  # @return [Problem]
  #[GetRoute("/unknown")]
  def get_unknown
    raise ArgumentError, PRIVATE_MESSAGE
  end

  # @return [Problem]
  #[GetRoute("/broken")]
  def get_broken
    raise OrderBroken, PRIVATE_MESSAGE
  end

  # @return [Problem]
  #[GetRoute("/invalid")]
  def get_invalid
    raise OrderInvalid, PRIVATE_MESSAGE
  end
end
"#,
            ),
            (
                "controllers/StatusController",
                r#"#[RequestRoute("/status")]
#[Public]
class StatusController
  # @return [Problem]
  #[GetRoute]
  def get_status
    raise OrderNotFound, "PRIVATE_ERROR_SENTINEL"
  end
end
"#,
            ),
            (
                "controllers/OrdersErrorHandler",
                r#"#[ErrorHandler]
class OrdersErrorHandler
  PRIVATE_MESSAGE = "PRIVATE_ERROR_SENTINEL"
  # @param error [OrderNotFound]
  # @return [Problem]
  #[HandlesError]
  #[ResponseStatus(404)]
  def missing(error)
    Problem.new(code: "missing")
  end

  # @param error [OrderGone]
  # @return [Problem]
  #[HandlesError]
  #[ResponseStatus(410)]
  def gone(error)
    Problem.new(code: "gone")
  end

  # @param error [OrderBroken]
  # @param request [Hash]
  # @return [Problem]
  #[HandlesError]
  #[ResponseStatus(503)]
  def broken(error, request)
    raise ArgumentError, PRIVATE_MESSAGE
  end

  # @param error [OrderInvalid]
  # @return [Problem]
  #[HandlesError]
  #[ResponseStatus(422)]
  def invalid(error)
    { "wrong" => PRIVATE_MESSAGE }
  end
end
"#,
            ),
        ],
        checks_invalid_model: true,
    };
}

#[tokio::test]
async fn error_handlers_answer_every_scenario() {
    Project::RUBY.verify().await;
}
