#[HTTPClient("/api")]
class OverrideClient
  # @return [Greeting]
  #[GetRequest("/never")]
  def read
  end
end
