@Store(BillingEntity.class, {"fields":["id"],"defaultSize":20,"maxSize":100})
interface BillingStore {
  Object existsById(long id)
}
