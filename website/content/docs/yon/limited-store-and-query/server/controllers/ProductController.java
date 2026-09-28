import models.ProductReport;

@RequestRoute("/products")
@Public
class ProductController {
    @GetRoute
    ProductReport getProduct() {
        Behaviors.Options options = new Behaviors.Options("public-catalog", new Behaviors.Signal());
        Behaviors.Context context = Behaviors.create("CatalogSetup", options);
        ProductStore products = new ProductStore(context);
        String activeStatus = "active";
        Object active = products.findByStatus(activeStatus);
        Object activeCount = products.countByStatus(activeStatus);
        java.util.Map<String, Object> page = Behaviors.map(products.bounded(java.util.Map.of("size", 5)));
        Object others = products.excluding(activeStatus, page);
        context.close();
        return new ProductReport(active, activeCount, others);
    }
}
