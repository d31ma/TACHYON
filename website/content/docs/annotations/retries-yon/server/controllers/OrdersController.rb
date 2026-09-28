#[RequestRoute("/orders")]
#[Public]
class OrdersController
  #[Needs]
  # @return [OrdersService]
  attr_reader :orders_service

  # @return [OrderReport]
  #[GetRoute]
  def get_orders
    id = "A-1"
    submitted = orders_service.submit(id)
    report = orders_service.report(id)
    OrderReport.new(submitted: submitted, report: report)
  end
end
