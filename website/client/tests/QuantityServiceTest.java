@Test(SiteTestSetup.class, {"partition":"tests"})
class QuantityServiceTest {
    void testOneThingIsSingular() {
        Assert.equals("1 feature", new QuantityService().describe(1, "feature"), "one feature is singular");
    }

    void testOtherCountsArePlural() {
        Assert.equals("12 features", new QuantityService().describe(12, "feature"), "twelve features are plural");
        Assert.equals("0 guides", new QuantityService().describe(0, "guide"), "no guides are plural");
    }
}
