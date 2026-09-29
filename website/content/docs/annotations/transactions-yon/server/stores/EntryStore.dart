@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
abstract interface class EntryStore {
  Future<Object?> findByStatus(String status);

  Future<Object?> countByStatus(String status);
}
