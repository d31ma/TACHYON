open System.Net
open System.Net.Http
open System.Text

[<RequestRoute("/products")>]
[<Public>]
type ProductsController() =
    [<GetRoute>]
    member this.GetProducts() : HttpResponseMessage =
        let body = new StringContent("<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>", Encoding.UTF8, "text/html")
        new HttpResponseMessage(HttpStatusCode.OK, Content = body)
