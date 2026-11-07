[Test(typeof(CatalogTestSetup), {"partition":"tests"})]
sealed class CatalogServiceTest {
    int twice(int value) { return value * 2; }
    void testHasAContext() {
        Assert.isTrue(behaviors != null, "the case has a context");
    }
    void testDoublesAValue() {
        Assert.equals(8, twice(4));
    }
}
