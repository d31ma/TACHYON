import models.ProbeReady

@Service
class ProbeService {
  var last: ProbeReady? = null

  @OnEvent
  fun observe(ready: ProbeReady) { last = ready }

  @OnStop
  fun destroy() { last = null }
}
