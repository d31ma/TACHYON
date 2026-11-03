from typing import Annotated
from models import ProbeReady


@service
class PublishService:
    application_event_publisher: Annotated[ApplicationEventPublisher, needs]

    @on_start
    def initialize(self):
        self.application_event_publisher.publish_event(ProbeReady(7))
