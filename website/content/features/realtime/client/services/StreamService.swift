@Service
final class StreamService {
    @Needs
    private let ticksClient: TicksClient
    var ticks: [Tick] = []

    @OnEvent
    func addTick(_ tick: Tick) {
        ticks = Array((ticks + [tick]).suffix(tickLimit()))
    }

    private func tickLimit() -> Int { 128 }
}
