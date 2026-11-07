#pragma once

class [[HTTPClient, CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})]] ProductsClient {
public:
    [[Cached]] [[GetRequest("/api/products")]]
    Task<tachyon::json::JsonNode> list();

    [[ClearCache({"method":"list"})]] [[PostRequest("/api/products")]]
    Task<tachyon::json::JsonNode> save([[RequestBody]] tachyon::json::JsonNode product);
};
