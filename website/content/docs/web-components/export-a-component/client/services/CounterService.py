from models import CountChanged


@service
class CounterService:
    count: int = 0
    application_event_publisher: ApplicationEventPublisher

    def add(self) -> None:
        self.count += 1
        self.application_event_publisher.publish_event(CountChanged(self.count), False)
