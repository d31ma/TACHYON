/** @import { ProbeReady } from '/models/ProbeReady.ts' */
@RequestRoute("/probe")
@Public
class ProbeController {
  /** @type {ProbeService} */
  @Needs
  probeService;
  /** @type {PublishService} */
  @Needs
  publishService;
  /** @returns {ProbeReady | null} */
  @GetRoute
  getProbe() { return this.probeService.last; }
}
