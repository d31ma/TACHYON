#pragma once
#include <string>
#include <vector>

class [[Service]] OrderService {
public:
    std::vector<std::string> list() const {
        return {"anvil"};
    }
};
