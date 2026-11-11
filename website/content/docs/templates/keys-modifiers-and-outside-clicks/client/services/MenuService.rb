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

  # @return [void]
  def close
    @expanded = false
  end
end
