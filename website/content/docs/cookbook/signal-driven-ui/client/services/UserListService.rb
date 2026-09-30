#[Service]
class UserListService
  # @return [Array<User>]
  attr_reader :users

  def initialize
    @users_client = UsersClient.new
    @users = []
  end

  # @return [void]
  #[OnStart]
  def start
    refresh
  end

  # @param changed [UsersChanged]
  # @return [void]
  #[OnEvent]
  def users_changed(changed)
    refresh_when_saved(changed)
  end

  private

  # @return [void]
  def refresh
    @users = @users_client.list.map { |user| User.new(name: user["name"]) }
  end

  # @param changed [UsersChanged]
  # @return [void]
  def refresh_when_saved(changed)
    refresh if changed.saved
  end
end
