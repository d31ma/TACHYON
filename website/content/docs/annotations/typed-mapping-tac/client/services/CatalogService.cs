using Models;
[Service]
public sealed class CatalogService {
    [Needs]
    ProductConverter _productConverter;
    public string result = "";

    public void load() {
        var detail = new Detail("detail", "detail-private");
        var product = _productConverter.copy(new Product(7, "Anvil", detail, new[] {detail}, "product-private"));
        result = Text(_productConverter.map(product));
    }

    private static string Text(Summary summary) {
        return $"{summary.kind} {summary.id}: {summary.name}, {summary.detail.name}, {summary.items.Length}";
    }
}
