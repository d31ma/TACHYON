class Endpoint {
    @Setting("${yon.server.port}")
    var port: Long = 0
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    var redis: String = ""
}
