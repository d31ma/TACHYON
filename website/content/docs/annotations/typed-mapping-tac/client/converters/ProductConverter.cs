using Models;
[Converter]
public interface ProductConverter {
    [Convert({"fields":[{"target":"kind","constant":"product"}]})]
    Summary map(Product product);

    Product copy(Product product);
}
