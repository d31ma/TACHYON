#pragma once
#include <string>
#include "../models/Identity.hpp"

class [[RequestRoute("/me")]] [[SignedIn]] MeController {
public:
    [[GetRoute]]
    std::string getMe([[FromIdentity]] const Identity& caller) {
        return caller.user;
    }
};
