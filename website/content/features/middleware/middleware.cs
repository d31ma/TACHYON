sealed class Middleware {
    public static HttpResponseMessage before(HttpRequestMessage request) {
        var token = request.Headers.Authorization?.ToString();
        if (request.RequestUri.AbsolutePath.StartsWith("/private") && token != "let-me-in") {
            var body = new StringContent("{\"error\":\"Unauthorized\"}", System.Text.Encoding.UTF8, "application/json");
            return new HttpResponseMessage(System.Net.HttpStatusCode.Unauthorized) { Content = body };
        }
        return null;
    }

    public static HttpResponseMessage after(HttpRequestMessage request) {
        var response = new HttpResponseMessage(System.Net.HttpStatusCode.NoContent);
        response.Headers.Add("x-served-by", "tachyon");
        return response;
    }
}
