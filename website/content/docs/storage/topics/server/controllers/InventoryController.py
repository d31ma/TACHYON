from typing import Annotated
from models import InventoryRecord


@request_route("/inventory")
@public
@needs_all()
class InventoryController:
    application_event_publisher: ApplicationEventPublisher

    @post_route
    def post_record(self, record: Annotated[InventoryRecord, from_body]) -> InventoryRecord:
        self.application_event_publisher.publish_event(record)
        return record
