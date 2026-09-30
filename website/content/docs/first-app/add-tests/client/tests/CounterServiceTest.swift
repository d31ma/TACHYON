@Test(HelloTestSetup, {"partition":"tests"})
final class CounterServiceTest {
    func testIncrementAddsOne() async throws {
        let counter = CounterService()
        counter.increment()
        try Assert.equals(1, counter.count)
    }
}
