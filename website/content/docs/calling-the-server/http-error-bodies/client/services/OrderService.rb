#[Service]
class OrderService
  #[Needs]
  # @return [OrderClient]
  attr_reader :order_client

  # @return [Integer]
  attr_reader :status, :mistakes
  # @return [String]
  attr_reader :title

  def initialize
    @status = 0
    @title = ""
    @mistakes = 0
  end

  # @return [void]
  def send
    error = failure
    show(BehaviorError.status_of(error), problem_of(error))
  end

  private

  # @return [BehaviorError]
  def failure
    order_client.send_order({ "name" => "", "level" => 12 })
    BehaviorError.new("none", "no failure", nil)
  rescue BehaviorError => error
    error
  end

  # @param error [BehaviorError]
  # @return [Problem]
  def problem_of(error)
    BehaviorError.body_of(error, Problem) || Problem.new(title: "", status: 0, errors: [])
  end

  # @param status [Integer]
  # @param problem [Problem]
  # @return [void]
  def show(status, problem)
    @status = status
    @title = problem.title
    @mistakes = problem.errors.length
  end
end
