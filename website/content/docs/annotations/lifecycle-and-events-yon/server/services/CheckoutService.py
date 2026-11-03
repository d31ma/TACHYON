from typing import Annotated
from models import PlacedOrder


@service
class CheckoutService:
    application_event_publisher: Annotated[ApplicationEventPublisher, needs]

    @on_start
    def start(self) -> None:
        self.application_event_publisher.publishEvent(PlacedOrder("paid", 25))

    @on_stop
    def stop(self) -> None:
        pass
