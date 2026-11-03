@Test(HelloTestSetup, {"partition":"tests"})
final class CounterServiceTest {
    @Fake
    var greetingClient: GreetingClient? = nil

    func testIncrementAddsOne() async throws {
        let counter: CounterService = try construct()
        counter.increment()
        try Assert.equals(1, counter.count)
    }

    func testAskServerSavesTheMessage() async throws {
        let counter: CounterService = try construct()
        try await counter.askServer()
        try Assert.equals("Hi", counter.message)
    }
}
