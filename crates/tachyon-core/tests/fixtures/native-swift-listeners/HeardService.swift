@Service
class HeardService {
    var heard: String = ""
    let applicationEventPublisher: ApplicationEventPublisher

    func send(_ step: Int) { applicationEventPublisher.publishEvent(Ping(step: step)) }

    @OnEvent
    func later(_ ping: Ping) async {
        try? await Task.sleep(nanoseconds: 20_000_000)
        heard += "a\(ping.step)"
    }

    @OnEvent
    func now(_ ping: Ping) { heard += "s\(ping.step)" }
}
