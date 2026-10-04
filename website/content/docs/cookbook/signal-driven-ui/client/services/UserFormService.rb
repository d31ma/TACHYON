#[Service]
class UserFormService
  # @return [String]
  attr_accessor :name

  def initialize
    @users_client = UsersClient.new
    @events = ApplicationEventPublisher.new
    @name = ""
  end

  # @return [void]
  def save
    answer = @users_client.save(@name)
    @events.publish_event(UserSaved.new(name: answer["name"]))
  end
end
