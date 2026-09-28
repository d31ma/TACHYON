#[RequestRoute("/products")]
#[Public]
class ProductsController
  #[GetRoute]
  def get_products
    signal = BehaviorSignal.new
    values = { "YON_CATALOG_SIZE" => "25", "YON_CATALOG_ENABLED" => "true" }
    HandlerChecks.check_configuration(values)
    HandlerChecks.check_value_limits
    HandlerChecks.check_validation
    HandlerChecks.check_inactive_configuration(signal)
    context = Behaviors.create(AdaptersSetup, partition: "catalog", signal: signal, profiles: ["test"], properties: values)
    result = HandlerChecks.exercise(context, signal)
    context.dispose
    result
  end
end
