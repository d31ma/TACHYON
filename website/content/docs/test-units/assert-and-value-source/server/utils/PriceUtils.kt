object PriceUtils {
    fun withTax(cents: Long): Long {
        requirePositive(cents)
        return Math.round(cents * 1.2)
    }

    private fun requirePositive(cents: Long) {
        require(cents >= 0) { "A price is never negative." }
    }
}
