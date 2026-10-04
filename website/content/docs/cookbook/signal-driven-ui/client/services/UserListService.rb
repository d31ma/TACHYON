#[Service]
class UserListService
  # @return [Array<String>]
  attr_reader :names

  def initialize
    @names = []
  end

  # @param saved [UserSaved]
  # @return [void]
  #[OnEvent]
  def user_saved(saved)
    @names = @names + [saved.name]
  end
end
