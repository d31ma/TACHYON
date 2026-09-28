import models.ProbeReady

@RequestRoute("/probe")
@Public
class ProbeController {
  @Needs
  lateinit var probe: ProbeService
  @Needs
  lateinit var publish: PublishService
  @GetRoute
  fun getProbe(): ProbeReady? { return probe.last }
}
