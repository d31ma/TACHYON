from typing import Annotated

@service
class TicketService:
    clock: Annotated[Clock, needs]
    secure_random: Annotated[SecureRandom, needs]
    ticket: str = ''
    issued_at: float = 0.0

    def issue(self) -> None:
        self.ticket = self.secure_random.uuid()
        self.issued_at = self.clock.now_ms()
