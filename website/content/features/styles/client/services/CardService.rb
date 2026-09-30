#[Service]
class CardService
  # @return [String]
  attr_reader :name

  def initialize
    @name = "Anvil"
  end
end
