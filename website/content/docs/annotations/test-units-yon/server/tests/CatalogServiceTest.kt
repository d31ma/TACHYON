@Test(CatalogTestSetup::class, {"partition":"tests"})
class CatalogServiceTest {
    fun testOpensAContext() {
        Assert.isTrue(behaviors != null, "the case has a context")
    }
    fun twice(value: Int): Int = value * 2
    fun testDoublesAValue() {
        Assert.equals(8, twice(4))
    }
}
