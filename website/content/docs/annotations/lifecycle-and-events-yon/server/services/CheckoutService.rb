#[Service]
class CheckoutService
  # @return [void]
  #[OnStart]
  def start
    ApplicationEventPublisher.new.publish_event(PlacedOrder.new(status: "paid", total: 25))
  end

  # @return [void]
  #[OnStop]
  def stop
  end
end
