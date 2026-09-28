import models.ProbeReady

@RequestRoute("/probe")
@Public
class ProbeController {
  @Needs
  ProbeService probe
  @Needs
  PublishService publish
  @GetRoute
  ProbeReady getProbe() { return probe.last }
}
