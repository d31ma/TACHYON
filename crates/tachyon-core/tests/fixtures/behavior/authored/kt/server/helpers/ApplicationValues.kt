class ApplicationValues {
    @Setting("${yon.server.port:8080}")
    var port: Long = 0
    @Setting({"path":"yon.app.count","type":"integer","default":5,"min":1,"max":100})
    var count: Long = 0
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    var redis: String = ""
}

