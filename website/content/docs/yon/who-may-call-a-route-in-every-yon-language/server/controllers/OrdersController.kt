@RequestRoute("/orders")
@SignedIn
class OrdersController {
    @GetRoute
    @Public
    fun getOrders(): List<String> {
        return listOf("lamp", "desk")
    }

    @PostRoute
    fun postOrder(): List<String> {
        return listOf("chair")
    }

    @DeleteRoute
    @Allow(Admin::class)
    fun deleteOrders(): List<String> {
        return listOf()
    }
}
