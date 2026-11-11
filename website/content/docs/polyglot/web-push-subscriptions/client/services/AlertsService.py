from typing import Annotated, ClassVar

@service
class AlertsService:
    SERVER_KEY: ClassVar[str] = 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U'
    push: Annotated[Push, needs]
    endpoint: str = ''
    problem: str = ''

    def turn_on(self) -> None:
        try:
            self.push.subscribe(self.SERVER_KEY)
        except RuntimeError as refusal:
            self.problem = str(refusal)

    def turn_off(self) -> None:
        try:
            self.push.unsubscribe()
        except RuntimeError as refusal:
            self.problem = str(refusal)

    @on_event
    def subscribed(self, subscribed: PushSubscribed) -> None:
        self.endpoint = subscribed.endpoint

    @on_event
    def unsubscribed(self, unsubscribed: PushUnsubscribed) -> None:
        self.endpoint = ''

    @on_event
    def failed(self, failed: BrowserCallFailed) -> None:
        self.problem = failed.message
