import models.ProductReport;

@RequestRoute("/products")
@Public
class ProductController {
    @Needs
    ProductStore products;

    @GetRoute
    ProductReport getProduct() {
        String activeStatus = "active";
        Object active = products.findByStatus(activeStatus);
        Object activeCount = products.countByStatus(activeStatus);
        java.util.Map<String, Object> page = java.util.Map.of("size", 5);
        Object others = products.excluding(activeStatus, page);
        return new ProductReport(active, activeCount, others);
    }
}
