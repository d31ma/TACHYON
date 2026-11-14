#[IdentityProvider]
class AppIdentityProvider
  SESSIONS = {
    "ada-token" => Identity.new(user: "ada", roles: ["Admin"]),
    "bob-token" => Identity.new(user: "bob", roles: [])
  }.freeze
  BROKEN_TOKEN = "broken-token"

  # @param request [Net::HTTPGenericRequest]
  # @return [Identity, nil]
  def identify(request)
    token = request["authorization"].to_s
    refuse_broken(token)
    find(token)
  end

  private

  # @param token [String]
  # @return [void]
  def refuse_broken(token)
    raise IOError, "the session store is down" if token == BROKEN_TOKEN
  end

  # @param token [String]
  # @return [Identity, nil]
  def find(token)
    SESSIONS[token]
  end
end
