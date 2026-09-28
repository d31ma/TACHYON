#[Service]
class TallyService
  # @return [Integer]
  attr_reader :count
  # @return [String]
  attr_reader :runtime

  def initialize
    @count = 6
    @runtime = "Ruby, on the mruby web runtime"
  end

  # @return [Integer]
  def doubled
    @count * 2
  end
end