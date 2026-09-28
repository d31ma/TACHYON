@Test(CatalogTestSetup::class, {"partition":"tests"})
class CatalogServiceTest {
    fun testOpensAContext() {
        if (behaviors == null) throw IllegalStateException("no context")
    }
    fun twice(value: Int): Int = value * 2
    fun testDoublesAValue() {
        if (twice(4) != 8) throw IllegalStateException("expected 8")
    }
}
