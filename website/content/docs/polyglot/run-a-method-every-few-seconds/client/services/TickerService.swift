@Service
final class TickerService {
    var seconds: Int = 0

    @Every("1s")
    func count() {
        seconds += 1
    }
}
