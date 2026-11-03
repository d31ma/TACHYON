#[Service]
class CatalogService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
    @events = ApplicationEventPublisher.new
  end

  # @return [void]
  #[OnStart]
  def start
    announce
  end

  # @param value [CatalogCount]
  # @return [void]
  #[OnEvent]
  def changed(value)
    @count = value.count
  end

  # @return [void]
  #[OnStop]
  def cleanup
    @count = 0
  end

  private

  # @return [void]
  def announce
    @events.publish_event(CatalogCount.new(count: 5))
  end
end
