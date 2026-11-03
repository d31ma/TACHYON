@Service
class GuidePositionService {
    fun label(order: List<String>, slug: String): String = phrase(order.indexOf(slug) + 1, order.size)

    private fun phrase(position: Int, total: Int): String {
        if (position < FIRST) return ""
        return "Guide $position of $total"
    }

    companion object {
        private const val FIRST = 1
    }
}
