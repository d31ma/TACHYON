@Test(CatalogTestSetup.class, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest {
    void testOpensAContext() {
        Assert.isTrue(this.behaviors != null, "the case has a context")
    }
    int twice(int value) { return value * 2 }
    void testDoublesAValue() {
        Assert.equals(8, this.twice(4))
    }
}
