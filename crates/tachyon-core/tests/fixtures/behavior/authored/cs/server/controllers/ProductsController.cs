[RequestRoute("/products")]
[Public]
class ProductsController {
    [GetRoute]
    public HttpResponseMessage GetProducts() {
        return new HttpResponseMessage(System.Net.HttpStatusCode.OK) { Content = new StringContent(CatalogExercises.run().GetAwaiter().GetResult().ToString(), System.Text.Encoding.UTF8, "application/json") };
    }
}
