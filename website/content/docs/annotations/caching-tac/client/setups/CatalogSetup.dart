@Setup
class CatalogSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      caches: {'catalog': MemoryCacheHelper().adapter()},
      authorize: (call, execution) async => true,
    );
  }
}
