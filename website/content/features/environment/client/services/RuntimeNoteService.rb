#[Service]
class RuntimeNoteService
  # @return [String]
  attr_reader :runtime

  def initialize
    @runtime = "Ruby, on the mruby web runtime"
  end

  # @param development [Boolean]
  # @return [Boolean]
  def shows_notes(development)
    development
  end
end
