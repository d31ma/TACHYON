#[HTTPClient("/api")]
class GreetingClient
  # @return [Greeting]
  #[GetRequest("/greeting")]
  def get_greeting
  end
end
