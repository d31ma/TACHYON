#[RequestRoute("/events")]
#[Public]
class EventsController
  #[Needs]
  # @return [TicketService]
  attr_reader :ticket_service

  # @return [void]
  #[StreamRoute]
  def stream_events
    tickets_for(ticket_service.levels) { |ticket| yield ticket }
  end

  private

  # @param levels [Array<Integer>]
  # @return [Array<Integer>]
  def tickets_for(levels)
    levels.each do |level|
      yield(Ticket.new(sku: "anvil", level: level))
    end
  end
end
