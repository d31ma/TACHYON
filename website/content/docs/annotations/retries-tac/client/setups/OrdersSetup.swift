@Setup
final class OrdersSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.authorize = { _, _ in true }
        return adapters
    }
}
