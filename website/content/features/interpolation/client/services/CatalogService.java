import models.Product;

@Service
class CatalogService {
    String title = "Catalog";
    int count = 6;
    Product product = new Product("Anvil", "£42.00");
}
