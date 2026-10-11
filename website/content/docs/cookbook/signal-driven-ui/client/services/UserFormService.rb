#[Service]
class UserFormService
  #[Needs]
  # @return [UsersClient]
  attr_reader :users_client

  # @return [String]
  attr_accessor :name

  def initialize
    @events = ApplicationEventPublisher.new
    @name = ""
  end

  # @return [void]
  def save
    answer = users_client.save(@name)
    @events.publish_event(UserSaved.new(name: answer["name"]))
  end
end
