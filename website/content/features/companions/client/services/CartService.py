from models import CartTotal


@service
class CartService:
    total: int = 0
    application_event_publisher: ApplicationEventPublisher

    def add(self) -> None:
        self.total += 1
        self.application_event_publisher.publish_event(CartTotal(self.total), True)
