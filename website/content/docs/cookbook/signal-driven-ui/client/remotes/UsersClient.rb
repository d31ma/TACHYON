#[HTTPClient]
class UsersClient
  # @param name [String]
  # @return [Hash]
  #[PostRequest("/users")]
  def save(
    #[RequestBody]
    name
  )
  end
end
