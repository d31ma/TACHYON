#[Service]
class DrawerService
  # @return [Boolean]
  attr_reader :open

  def initialize
    @open = false
  end

  # @return [Boolean]
  def toggle
    @open = !@open
  end

  # @return [Boolean]
  def close
    @open = false
  end

  # @param key [String]
  # @return [void]
  def dismiss(key)
    close_on_escape(key)
  end

  private

  # @param key [String]
  # @return [void]
  def close_on_escape(key)
    close if key == "Escape"
  end
end
