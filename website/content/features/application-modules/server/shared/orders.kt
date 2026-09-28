object OrderService {
    fun place(item: String): Map<String, String> = mapOf("placed" to item)
}
