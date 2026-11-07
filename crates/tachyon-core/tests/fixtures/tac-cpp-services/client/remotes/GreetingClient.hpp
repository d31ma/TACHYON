#pragma once
#include "../models/Greeting.hpp"

class [[HTTPClient("/api")]] GreetingClient {
public:
    [[GetRequest("/greeting")]]
    Task<Greeting> read();

    [[PostRequest("/echo/{id}")]]
    Task<Greeting> echo(int id, [[RequestBody]] const Greeting& greeting);

    [[GetRequest("/missing")]]
    Task<Greeting> lost();
};
