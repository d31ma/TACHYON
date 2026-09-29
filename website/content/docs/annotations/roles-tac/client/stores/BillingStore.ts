@Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
export default class BillingStore {
  async existsById(id: number): Promise<boolean | void> {}
}
