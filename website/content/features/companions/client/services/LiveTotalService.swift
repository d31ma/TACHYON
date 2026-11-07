@Service
final class LiveTotalService {
    var seen: Int = 0

    @OnEvent({"replay":true})
    func changed(_ changed: CartTotal) { seen = changed.total }
}
