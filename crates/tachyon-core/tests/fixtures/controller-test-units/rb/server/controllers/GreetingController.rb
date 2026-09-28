#[RequestRoute("/api/greeting")]
#[Public]
class GreetingController
  #[Needs]
  attr_reader :greeting_service

  #[GetRoute]
  def get_greeting
    { "message" => greeting_service.message }
  end
end
