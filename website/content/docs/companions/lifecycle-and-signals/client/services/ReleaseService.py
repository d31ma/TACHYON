from models import ReleaseChanged


@service
class ReleaseService:
    release: str = ""
    application_event_publisher: ApplicationEventPublisher

    def announce(self) -> None:
        self.application_event_publisher.publish_event(ReleaseChanged("TACHYON"), True)

    @on_event({"replay":true})
    def receive(self, changed: ReleaseChanged) -> None:
        self.release = changed.release
