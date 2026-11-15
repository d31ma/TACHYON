#[RequestRoute("/me")]
#[SignedIn]
class MeController
  # @param caller [Identity]
  # @return [String]
  #[GetRoute]
  def get_me(
    #[FromIdentity]
    caller
  )
    caller.user
  end
end
