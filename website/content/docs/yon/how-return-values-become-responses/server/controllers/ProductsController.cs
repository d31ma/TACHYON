using System.Net;
using System.Net.Http;
using System.Text;

[RequestRoute("/products")]
[Public]
class ProductsController {
    [GetRoute]
    public HttpResponseMessage GetProducts() {
        var body = new StringContent("<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>", Encoding.UTF8, "text/html");
        return new HttpResponseMessage(HttpStatusCode.OK) { Content = body };
    }
}
