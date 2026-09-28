import models.ProductReport

@RequestRoute("/products")
@Public
class ProductController {
    @GetRoute
    fun getProduct(): ProductReport {
        val options = Behaviors.Options("public-catalog", Behaviors.Signal())
        val context = Behaviors.create("CatalogSetup", options)
        val products = ProductStore(context)
        val activeStatus = "active"
        val active = products.findByStatus(activeStatus)
        val activeCount = products.countByStatus(activeStatus)
        val page = products.bounded(mapOf("size" to 5)) as Map<String, Any?>
        val others = products.excluding(activeStatus, page)
        context.close()
        return ProductReport(active, activeCount, others)
    }
}
