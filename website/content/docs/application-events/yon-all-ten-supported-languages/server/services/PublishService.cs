[Service]
class PublishService {
  [Needs]
  readonly ApplicationEventPublisher _applicationEventPublisher;

  [OnStart]
  public void Initialize() { _applicationEventPublisher.PublishEvent(new Models.ProbeReady(7)); }
}
