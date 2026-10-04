#pragma once
#include <string>
#include <vector>
#include "../models/UserSaved.hpp"

class [[Service]] UserListService {
public:
    std::vector<std::string> names;

    [[OnEvent]]
    void userSaved(const UserSaved& saved) { names.push_back(saved.name); }
};
