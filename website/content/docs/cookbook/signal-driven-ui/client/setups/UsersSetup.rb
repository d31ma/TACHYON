#[Setup]
class UsersSetup
  # @param _execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(_execution)
    exchange = lambda do |request, _running|
      { "name" => request["body"] }
    end
    { "http" => { "users" => { "exchange" => exchange } } }
  end
end
