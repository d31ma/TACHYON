from typing import Annotated


@Test(HelloTestSetup, {"partition":"tests"})
class CounterServiceTest:
    greeting_client: Annotated[GreetingClient, fake]

    def test_increment_adds_one(self):
        counter = CounterService(self.greeting_client)
        counter.increment()
        Assert.equals(1, counter.count)

    async def test_ask_server_saves_the_message(self):
        self.greeting_client = GreetingClient()
        self.greeting_client.behaviors = self.behaviors
        counter = CounterService(self.greeting_client)
        await counter.ask_server()
        Assert.equals('Hi', counter.message)
