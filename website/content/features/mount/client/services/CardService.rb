#[Service]
class CardService
  # @return [String]
  attr_reader :name
  # @return [String]
  attr_reader :status

  def initialize
    @name = "Anvil"
    @status = "waiting"
  end

  # @return [void]
  #[OnStart]
  def activate
    @status = "active"
  end
end
