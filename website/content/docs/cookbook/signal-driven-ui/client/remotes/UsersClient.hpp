#pragma once
#include <string>
#include "../models/UserSaved.hpp"

class [[HTTPClient]] UsersClient {
public:
    [[PostRequest("/users")]]
    Task<UserSaved> save([[RequestBody]] std::string name);
};
