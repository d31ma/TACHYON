import models.ProductReport

@RequestRoute("/products")
@Public
class ProductController {
    @Needs
    val products: ProductStore

    @GetRoute
    fun getProduct(): ProductReport {
        val activeStatus = "active"
        val active = products.findByStatus(activeStatus)
        val activeCount = products.countByStatus(activeStatus)
        val others = products.excluding(activeStatus, mapOf("size" to 5))
        val first = products.findById(1)
        return ProductReport(active, activeCount, others, first)
    }
}
