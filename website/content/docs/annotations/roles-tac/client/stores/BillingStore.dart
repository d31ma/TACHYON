@Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
abstract interface class BillingStore {
  Future<Object?> existsById(int id);
}
