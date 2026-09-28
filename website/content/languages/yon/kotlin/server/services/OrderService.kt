@Service
class OrderService {
    companion object {
    fun list(): List<String> = OrderStore.all()
}
}
