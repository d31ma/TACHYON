/** @import { ProbeReady } from '/models/ProbeReady.ts' */
@Service
class ProbeService {
  /** @type {ProbeReady | null} */
  last = null;

  /** @param {ProbeReady} ready */
  @OnEvent
  observe(ready) { this.last = ready; }

  @OnStop
  destroy() { this.last = null; }
}
