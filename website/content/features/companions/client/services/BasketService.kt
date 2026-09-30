@Service
class BasketService {
    var count: Int = 0

    fun doubled(): Int = count * 2

    @OnStart
    fun initialize() { count = 6 }
}
