<?php
#[Service]
class ProbeService {
  public ?\Models\ProbeReady $last = null;

  #[OnEvent]
  function observe(\Models\ProbeReady $ready): void { $this->last = $ready; }

  #[OnStop]
  function destroy(): void { $this->last = null; }
}
