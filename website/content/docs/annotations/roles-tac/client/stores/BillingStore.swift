@Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
protocol BillingStore {
    func existsById(_ id: BehaviorJSON) async throws -> BehaviorJSON
}
