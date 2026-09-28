@service
class OrdersService:
    behaviors: object = None
    _FIRST_WORKING_ATTEMPT = 3
    _attempts = 0

    @on_start
    async def open(self) -> None:
        self.behaviors = await Behaviors.create(OrdersSetup, {'partition': 'orders', 'signal': BehaviorSignal()})

    @retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    async def submit(self, id: str) -> dict[str, object]:
        self._attempts += 1
        self._fail_first_two_attempts()
        return self._report_of(id)

    @max_at_once({"limit":2,"policy":"block"})
    async def report(self, id: str) -> dict[str, object]:
        return self._report_of(id)

    def _report_of(self, id: str) -> dict[str, object]:
        return {'id': id, 'attempts': self._attempts}

    def _fail_first_two_attempts(self) -> None:
        if self._attempts < self._FIRST_WORKING_ATTEMPT:
            raise RuntimeError('the payment gateway is unavailable')
