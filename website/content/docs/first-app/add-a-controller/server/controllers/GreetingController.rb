#[RequestRoute("/api/greeting")]
#[Public]
class GreetingController
  # @return [Greeting]
  #[GetRoute]
  def get_greeting
    Greeting.new(message: "Hello from Yon")
  end
end
