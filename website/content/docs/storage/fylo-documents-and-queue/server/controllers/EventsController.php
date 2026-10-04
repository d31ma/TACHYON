<?php
#[RequestRoute("/events")]
#[Public]
class EventsController {
  #[Needs]
  private EventService $eventService;
  #[GetRoute]
  public function getEvents(): array { return $this->eventService->report(); }
}
