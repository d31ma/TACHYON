<?php
class ApplicationValues {
    #[Setting('${yon.server.port:8080}')]
    public int $port = 0;
    #[Setting({"path":"yon.catalog.size","type":"integer","default":5,"min":1,"max":100})]
    public int $size = 0;
    #[Setting('${yon.redis.url:redis://127.0.0.1:6379}')]
    public string $redis = '';
}

