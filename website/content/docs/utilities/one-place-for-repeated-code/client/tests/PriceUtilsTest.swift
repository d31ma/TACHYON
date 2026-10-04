@Test(PriceTestSetup, {"partition":"tests"})
final class PriceUtilsTest {
    func testLabelShowsDollars() async throws {
        try Assert.equals("$12.50", PriceUtils.label(1250))
    }

    func testBasketUsesTheUtility() async throws {
        try Assert.equals("$12.50", BasketService().totalLabel())
    }
}
