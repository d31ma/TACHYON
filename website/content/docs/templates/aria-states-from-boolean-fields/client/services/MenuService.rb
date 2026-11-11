#[Service]
class MenuService
  # @return [Boolean]
  attr_reader :expanded

  def initialize
    @expanded = false
  end

  # @return [void]
  def toggle
    @expanded = !@expanded
  end
end
