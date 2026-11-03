open System.Net
open System.Net.Http
open System.Text

type Middleware =
    static member before(request: HttpRequestMessage) : HttpResponseMessage =
        let token = request.Headers.Authorization
        let signedIn = not (isNull token) && token.ToString() = "let-me-in"
        if request.RequestUri.AbsolutePath.StartsWith("/private") && not signedIn then
            let body = new StringContent("{\"error\":\"Unauthorized\"}", Encoding.UTF8, "application/json")
            new HttpResponseMessage(HttpStatusCode.Unauthorized, Content = body)
        else
            null

    static member after(request: HttpRequestMessage) : HttpResponseMessage =
        let response = new HttpResponseMessage(HttpStatusCode.NoContent)
        response.Headers.Add("x-served-by", "tachyon")
        response
