#[RequestRoute("/events")]
#[Public]
class EventsController
  #[Needs]
  # @return [EventService]
  attr_reader :event_service

  # @return [Hash]
  #[GetRoute]
  def get_events
    event_service.report
  end
end
