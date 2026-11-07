from models import PlacedOrder


@service
class CheckoutService:
    placed: int = 0
    application_event_publisher: ApplicationEventPublisher

    @on_start
    def _start(self) -> None:
        self.place()

    def place(self) -> None:
        self.placed += 1
        self.application_event_publisher.publish_event(PlacedOrder("paid", 25), True)

    @on_stop
    def _stop(self) -> None:
        self.placed = 0
