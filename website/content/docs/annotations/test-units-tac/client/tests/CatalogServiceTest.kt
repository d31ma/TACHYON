@Test(CatalogTestSetup::class, {"partition":"tests"})
class CatalogServiceTest {
    fun twice(value: Int): Int = value * 2
    fun testHasAContext() {
        if (behaviors == null) throw IllegalStateException("no context")
    }
    fun testDoublesAValue() {
        if (twice(4) != 8) throw IllegalStateException("expected 8")
    }
}
