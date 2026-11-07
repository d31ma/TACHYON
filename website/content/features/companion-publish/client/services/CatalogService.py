from models import CatalogCount


@service
class CatalogService:
    count: int = 0
    application_event_publisher: ApplicationEventPublisher

    @on_start
    def initialize_count(self) -> None:
        self._announce()

    @on_event
    def changed(self, value: CatalogCount) -> None:
        self.count = value.count

    @on_stop
    def cleanup(self) -> None:
        self.count = 0

    def _announce(self) -> None:
        self.application_event_publisher.publish_event(CatalogCount(5))
