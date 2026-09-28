using Models;

[RequestRoute("/probe")]
[Public]
class ProbeController {
  [Needs]
  public ProbeService Probe;
  [Needs]
  public PublishService Publish;
  [GetRoute]
  public ProbeReady? GetProbe() { return Probe.Last; }
}
