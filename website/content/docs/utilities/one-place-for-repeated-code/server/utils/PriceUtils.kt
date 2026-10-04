object PriceUtils {
    fun label(cents: Long): String = "$" + dollars(cents)

    private fun dollars(cents: Long): String = java.math.BigDecimal.valueOf(cents, 2).toPlainString()
}
