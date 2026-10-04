#[HTTPClient({"adapter":"publicConfiguration"})]
class ApplicationClient
  # @return [Hash]
  #[GetRequest("/manifest.webmanifest")]
  def load
  end
end
