#[HTTPClient]
class UsersClient
  # @return [Array<Hash>]
  #[GetRequest("/users")]
  def list()
  end

  # @param form [Hash]
  # @return [Hash]
  #[PostRequest("/users")]
  def save(
    #[RequestBody]
    form
  )
  end
end
