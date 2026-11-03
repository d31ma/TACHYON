@Test(CatalogTestSetup::class, {"partition":"tests"})
class CatalogServiceTest {
    fun twice(value: Int): Int = value * 2
    fun testHasAContext() {
        Assert.isTrue(behaviors != null, "the case has a context")
    }
    fun testDoublesAValue() {
        Assert.equals(8, twice(4))
    }
}
