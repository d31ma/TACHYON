#pragma once
#include <string>

struct Endpoint {
    [[Setting("${yon.server.port}")]] int port = 0;
    [[Setting("${yon.redis.url:redis://127.0.0.1:6379}")]] std::string redis;
};
