<?php
#[Service]
class AuditService {
  /** @var list<string> */
  public array $log = [];
  public int $settled = 0;

  #[OnEvent({"id":"audit-log"})]
  #[Order(-10)]
  function record(\Models\PlacedOrder $order): void { $this->log[] = $order->status; }

  #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
  #[Order(10)]
  function settle(\Models\PlacedOrder $order): \Models\SettledOrder { return new \Models\SettledOrder($order->total); }

  #[OnEvent]
  function saveSettled(\Models\SettledOrder $settled): void { $this->settled = $settled->total; }
}
