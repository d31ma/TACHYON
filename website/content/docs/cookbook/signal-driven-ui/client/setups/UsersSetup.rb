#[Setup]
class UsersSetup
  BODY = "body"

  # @param _execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(_execution)
    users = [{ "name" => "Ada" }]
    exchange = lambda do |request, _running|
      users << request[BODY] if request.key?(BODY)
      users
    end
    { "http" => { "users" => { "exchange" => exchange } } }
  end
end
