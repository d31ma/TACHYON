#pragma once
#include "../models/Greeting.hpp"

class [[HTTPClient("/api")]] OverrideClient {
public:
    [[GetRequest("/never")]]
    Task<Greeting> read();
};
