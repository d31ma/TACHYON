#pragma once
#include "../models/Greeting.hpp"

class [[HTTPClient("/api")]] GreetingClient {
public:
    [[GetRequest("/greeting")]]
    Task<Greeting> getGreeting();
};
