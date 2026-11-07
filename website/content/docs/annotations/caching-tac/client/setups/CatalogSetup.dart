@Setup
class CatalogSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      caches: {'catalog': MemoryCacheDelegate().adapter()},
      authorize: (call, execution) async => true,
    );
  }
}
