<?php
#[Service]
class PublishService {
  #[Needs]
  private ApplicationEventPublisher $applicationEventPublisher;

  #[OnStart]
  function initialize() { $this->applicationEventPublisher->publishEvent(new \Models\ProbeReady(7)); }
}
