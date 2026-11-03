using Models;

[Service]
public class ReleaseService {
    public string Release = "";
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public void Announce() {
        applicationEventPublisher.PublishEvent(new ReleaseChanged("TACHYON"), true);
    }

    [OnEvent({"replay":true})]
    public void Receive(ReleaseChanged changed) { Release = changed.Release; }
}
