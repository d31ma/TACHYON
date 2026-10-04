import models.Detail;
import models.Product;
import models.Summary;
@Service
class CatalogService {
    @Needs
    ProductConverter productConverter;
    String result = "";

    void load() {
        Detail detail = new Detail("detail", "detail-private");
        Product product = productConverter.copy(new Product(7, "Anvil", detail, java.util.List.of(detail), "product-private"));
        result = text(productConverter.map(product));
    }

    private String text(Summary summary) {
        return String.format("%s %d: %s, %s, %d", summary.kind(), summary.id(), summary.name(), summary.detail().name(), summary.items().size());
    }
}
