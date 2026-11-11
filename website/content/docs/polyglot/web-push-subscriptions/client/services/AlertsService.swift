@Service
final class AlertsService {
    private static let serverKey = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U"
    private let push = Push()
    var endpoint: String = ""
    var problem: String = ""

    func turnOn() {
        keepProblem { try push.subscribe(applicationServerKey: Self.serverKey) }
    }

    func turnOff() {
        keepProblem { try push.unsubscribe() }
    }

    @OnEvent
    func subscribed(_ subscribed: PushSubscribed) {
        endpoint = subscribed.endpoint
    }

    @OnEvent
    func unsubscribed(_ unsubscribed: PushUnsubscribed) {
        endpoint = ""
    }

    @OnEvent
    func failed(_ failed: BrowserCallFailed) {
        problem = failed.message
    }

    private func keepProblem(_ call: () throws -> Void) {
        do {
            try call()
        } catch {
            problem = String(describing: error)
        }
    }
}
