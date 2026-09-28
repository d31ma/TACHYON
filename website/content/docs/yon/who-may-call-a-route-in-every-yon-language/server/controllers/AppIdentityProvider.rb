#[IdentityProvider]
class AppIdentityProvider
  SESSIONS = {
    "ada-token" => Identity.new(user: "ada", roles: ["Admin"]),
    "bob-token" => Identity.new(user: "bob", roles: [])
  }.freeze

  # @param request [Net::HTTPGenericRequest]
  # @return [Identity, nil]
  def identify(request)
    find(request["authorization"].to_s)
  end

  private

  # @param token [String]
  # @return [Identity, nil]
  def find(token)
    SESSIONS[token]
  end
end
