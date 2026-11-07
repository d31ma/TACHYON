import type { ProbeReady } from '/models/ProbeReady.ts'

@RequestRoute("/probe")
@Public
class ProbeController {
  @Needs
  probeService!: ProbeService;
  @Needs
  publishService!: PublishService;
  @GetRoute
  getProbe(): ProbeReady | null { return this.probeService.last; }
}
