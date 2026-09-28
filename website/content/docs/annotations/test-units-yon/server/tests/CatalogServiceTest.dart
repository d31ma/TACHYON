@Test(CatalogTestSetup, {"partition":"tests"})
class CatalogServiceTest {
  Future<void> testOpensAContext() async {
    if (behaviors == null) throw StateError('no context');
  }

  int _twice(int value) => value * 2;

  Future<void> testDoublesAValue() async {
    if (_twice(4) != 8) throw StateError('expected 8');
  }
}
