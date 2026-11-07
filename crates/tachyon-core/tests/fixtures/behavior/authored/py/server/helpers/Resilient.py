from typing import Annotated

class Resilient:
    behaviors: Annotated[object, needs]
    attempts = 0
    @retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
    async def flaky(self):
        Resilient.attempts += 1
        if Resilient.attempts < 3:
            raise RuntimeError('transient')
        return Resilient.attempts
    @max_at_once({"limit":2,"policy":"block"})
    async def bounded(self, value):
        return value

