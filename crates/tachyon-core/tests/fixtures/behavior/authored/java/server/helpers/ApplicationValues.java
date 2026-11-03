class ApplicationValues {
    @Setting("${yon.server.port:8080}")
    long port;
    @Setting({"path":"yon.app.count","type":"integer","default":5,"min":1,"max":100})
    long count;
    @Setting("${yon.redis.url:redis://127.0.0.1:6379}")
    String redis;
}

