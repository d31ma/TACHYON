@Store(BillingEntity::class, {"fields":["id"],"defaultSize":20,"maxSize":100})
interface BillingStore {
  fun existsById(id: Long): Any?
}
