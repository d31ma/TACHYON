#pragma once
#include "../models/Greeting.hpp"

class [[HTTPClient("/api"), CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})]] GreetingClient {
public:
    [[Cached]] [[GetRequest("/greeting")]]
    Task<Greeting> read();

    [[ClearCache({"method":"read"})]] [[PostRequest("/clear")]]
    Task<Greeting> clear();
};
