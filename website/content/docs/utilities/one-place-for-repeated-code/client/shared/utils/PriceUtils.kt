object PriceUtils {
    fun label(cents: Int): String = "$" + dollars(cents)

    private fun dollars(cents: Int): String {
        val digits = cents.toString().padStart(3, '0')
        val split = digits.length - 2
        return digits.substring(0, split) + "." + digits.substring(split)
    }
}
