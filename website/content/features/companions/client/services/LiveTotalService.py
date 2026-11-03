from models import CartTotal


@service
class LiveTotalService:
    seen: int = 0

    @on_event({"replay":true})
    def changed(self, changed: CartTotal) -> None:
        self.seen = changed.total
