import typing
from models import Guide

@service
@needs_all
class CounterService:
    counter: CounterHelper
    title: str
    count: int
    fixed: typing.Final[str]
    current_guide: typing.Optional[Guide] = None
    echo_count: int = 0

    @on_start
    def _start(self) -> None:
        self.count += 1
        self.current_guide = Guide(title='initial', pages=1)

    def increment(self) -> int:
        self.count += self.counter.amount()
        return self.count

    def echo(self, value: Guide) -> Guide:
        self.echo_count += 1
        value._secret = 'must not leak'
        return value
