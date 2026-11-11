#[Service]
class RenameService
  # @return [Boolean]
  attr_reader :editing
  # @return [String]
  attr_reader :problem

  def initialize
    @focus = Focus.new
    @editing = false
    @problem = ""
  end

  # @return [void]
  def open
    @editing = true
    keep_problem { @focus.trap("dialog") }
    keep_problem { @focus.move("name") }
  end

  # @return [void]
  def close
    @editing = false
    keep_problem { @focus.release }
  end

  # @param failed [BrowserCallFailed]
  # @return [void]
  #[OnEvent]
  def failed(failed)
    @problem = failed.message
  end

  private

  # @return [void]
  def keep_problem
    yield
  rescue RuntimeError => refusal
    @problem = refusal.message
  end
end
