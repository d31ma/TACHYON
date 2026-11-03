@Test(CatalogTestSetup, {"partition":"tests"})
class CatalogServiceTest {
  Future<void> testOpensAContext() async {
    Assert.isTrue(behaviors != null, 'a context');
  }

  int _twice(int value) => value * 2;

  Future<void> testDoublesAValue() async {
    Assert.equals(8, _twice(4));
  }
}
