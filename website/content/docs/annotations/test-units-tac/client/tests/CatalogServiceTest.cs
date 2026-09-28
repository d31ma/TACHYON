[Test(typeof(CatalogTestSetup), {"partition":"tests"})]
sealed class CatalogServiceTest {
    int twice(int value) { return value * 2; }
    void testHasAContext() {
        if (this.behaviors == null) throw new System.Exception("no context");
    }
    void testDoublesAValue() {
        if (this.twice(4) != 8) throw new System.Exception("expected 8");
    }
}
