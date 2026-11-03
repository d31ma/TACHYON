@Test(PriceTestSetup.class, {"partition":"tests"})
class PriceUtilsTest {
    void testLabelShowsDollars() {
        Assert.equals("$12.50", PriceUtils.label(1250));
    }
}
