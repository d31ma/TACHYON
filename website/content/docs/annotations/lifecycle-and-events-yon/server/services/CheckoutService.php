<?php
#[Service]
class CheckoutService {
  #[Needs]
  private ApplicationEventPublisher $applicationEventPublisher;

  #[OnStart]
  function start(): void {
    $this->applicationEventPublisher->publishEvent(new \Models\PlacedOrder("paid", 25));
  }

  #[OnStop]
  function stop(): void {}
}
