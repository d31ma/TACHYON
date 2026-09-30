@Service
class TallyService {
    var count: Int = 0

    fun add(amount: Int) {
        count += amount
    }

    fun summary(unit: String): String = "$count $unit"
}
