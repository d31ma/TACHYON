type Endpoint() =
    [<Setting("${yon.server.port}")>]
    member val port : int64 = 0L with get, set

    [<Setting("${yon.redis.url:redis://127.0.0.1:6379}")>]
    member val redis : string = "" with get, set
