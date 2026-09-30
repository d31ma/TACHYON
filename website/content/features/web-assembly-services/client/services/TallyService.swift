@Service
final class TallyService {
    var count: Int = 0

    func add(_ amount: Int) {
        count += amount
    }

    func summary(_ unit: String) -> String {
        return "\(count) \(unit)"
    }
}
