#[Service]
class ReportService
  # @return [String]
  attr_accessor :problem

  def initialize
    @problem = ""
  end

  # @return [void]
  def minimize
    keep_problem { WindowControls.new.minimize }
  end

  # @return [void]
  def resize
    keep_problem { WindowControls.new.resize(1280, 800) }
  end

  # @return [void]
  def rename
    keep_problem { WindowControls.new.set_title("Quarterly report") }
  end

  private

  # @return [void]
  def keep_problem
    yield
  rescue RuntimeError => refusal
    @problem = refusal.message
  end
end
