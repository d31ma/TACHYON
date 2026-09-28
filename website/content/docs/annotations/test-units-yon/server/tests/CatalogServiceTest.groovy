@Test(CatalogTestSetup.class, {"partition":"tests"})
@UseProfiles("test")
class CatalogServiceTest {
    void testOpensAContext() {
        if (this.behaviors == null) throw new IllegalStateException("no context")
    }
    int twice(int value) { return value * 2 }
    void testDoublesAValue() {
        if (this.twice(4) != 8) throw new IllegalStateException("expected 8")
    }
}
