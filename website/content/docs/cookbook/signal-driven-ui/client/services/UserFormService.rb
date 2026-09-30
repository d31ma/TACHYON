#[Service]
class UserFormService
  # @return [User]
  attr_reader :form

  def initialize
    @users_client = UsersClient.new
    @events = ApplicationEventPublisher.new
    @form = User.new(name: "")
  end

  # @param name [String]
  # @return [void]
  def typed(name)
    @form = User.new(name: name)
  end

  # @return [void]
  def save
    @users_client.save(@form.to_h)
    @events.publish_event(UsersChanged.new(saved: true))
  end
end
