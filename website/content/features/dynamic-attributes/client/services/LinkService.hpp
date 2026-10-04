#pragma once
#include <string>
#include "../models/Topic.hpp"

class [[Service]] LinkService {
public:
    Topic topic{"routing", "Routing"};
    std::string state = "ready";
    std::string message = "Everything is up to date.";
};
