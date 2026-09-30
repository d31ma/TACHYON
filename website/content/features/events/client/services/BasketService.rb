#[Service]
class BasketService
  # @return [Integer]
  attr_accessor :count
  # @return [String]
  attr_accessor :label
  # @return [Array<Tab>]
  attr_reader :tabs

  def initialize
    @count = 0
    @label = ""
    @selected = ""
    @tabs = [Tab.new(id: "a", name: "A")]
  end

  # @return [void]
  def increment
    @count += 1
  end

  # @param id [String]
  # @return [void]
  def select(id)
    @selected = id
  end
end
