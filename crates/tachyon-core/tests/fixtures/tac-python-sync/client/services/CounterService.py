from __future__ import annotations
from typing import Final, Optional
from models import Guide

@service
@needs_all
class CounterService:
    counter: CounterHelper
    title: str
    count: int
    fixed: Final[str]
    current_guide: Optional[Guide]

    @on_start
    def _start(self) -> None:
        self.count += 1

    def increment(self) -> int:
        self.count += self.counter.amount()
        return self.count

    def echo(self, value: Guide) -> Guide:
        value._secret = 'must not leak'
        return value
