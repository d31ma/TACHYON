import type { ProbeReady } from '/models/ProbeReady';

@Service
class ProbeService {
  last: ProbeReady | null = null;

  @OnEvent
  observe(ready: ProbeReady) { this.last = ready; }

  @OnStop
  destroy() { this.last = null; }
}
