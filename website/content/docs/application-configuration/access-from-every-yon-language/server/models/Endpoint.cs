public sealed class Endpoint {
    [Setting("${yon.server.port}")]
    public long port;
    [Setting("${yon.redis.url:redis://127.0.0.1:6379}")]
    public string redis = "";
}
