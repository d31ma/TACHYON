#[Service]
class LinkService
  # @return [Topic]
  attr_reader :topic
  # @return [String]
  attr_reader :state
  # @return [String]
  attr_reader :message

  def initialize
    @topic = Topic.new(slug: "routing", title: "Routing")
    @state = "ready"
    @message = "Everything is up to date."
  end
end
