#[Service]
class EditorService
  # @return [String]
  #[SessionState]
  attr_accessor :draft
  # @return [String]
  #[LocalState]
  attr_accessor :theme

  def initialize
    @draft = ""
    @theme = "calm"
  end
end
