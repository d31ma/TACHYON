#[RequestRoute("/api/greeting")]
#[Public]
class GreetingController
  #[Needs]
  # @return [GreetingService]
  attr_reader :greeting_service

  # @return [Hash]
  #[GetRoute]
  def get_greeting
    { "message" => greeting_service.message }
  end
end
