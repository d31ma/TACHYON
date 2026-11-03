<?php
use Models\ProbeReady;

#[RequestRoute("/probe")]
#[Public]
class ProbeController {
  #[Needs]
  public ProbeService $probe;
  #[Needs]
  public PublishService $publish;
  #[GetRoute]
  function getProbe(): ?ProbeReady { return $this->probe->last; }
}
