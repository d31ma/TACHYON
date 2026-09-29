import type { ProbeReady } from '/models/ProbeReady.ts';

@Service
export default class ProbeService {
  last: ProbeReady | null = null;

  @OnEvent
  observe(ready: ProbeReady) { this.last = ready; }

  @OnStop
  destroy() { this.last = null; }
}
