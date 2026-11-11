@Service
class TickerService {
    var seconds: Int = 0

    @Every("1s")
    fun count() {
        seconds += 1
    }
}
