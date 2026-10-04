[Test(typeof(PriceTestSetup), {"partition":"tests"})]
sealed class PriceUtilsTest {
    void testLabelShowsDollars() {
        Assert.equals("$12.50", PriceUtils.Label(1250));
    }
}
