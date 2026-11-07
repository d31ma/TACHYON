@Service
final class BasketService {
    var count: Int = 0

    func doubled() -> Int { count * 2 }

    @OnStart
    func initialize() { count = 6 }
}
