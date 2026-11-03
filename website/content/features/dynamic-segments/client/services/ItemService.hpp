#pragma once
#include <string>

class [[Service]] ItemService {
public:
    std::string idFrom(const std::string& path) const {
        return path.substr(path.rfind('/') + 1);
    }
};
