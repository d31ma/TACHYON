#pragma once
#include <string>

class [[Service]] CardService {
public:
    std::string name = "Anvil";
    std::string status = "waiting";

    [[OnStart]]
    void activate() { status = "active"; }
};
