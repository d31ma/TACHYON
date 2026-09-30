import models.ReleaseChanged;

@Service
class ReleaseService {
    String release = "";
    ApplicationEventPublisher applicationEventPublisher;

    void announce() {
        applicationEventPublisher.publishEvent(new ReleaseChanged("TACHYON"), true);
    }

    @OnEvent({"replay":true})
    void receive(ReleaseChanged changed) { release = changed.release(); }
}
