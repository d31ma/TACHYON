#pragma once
#include <string>
#include <vector>

struct Lifecycle {
    std::vector<std::string> log;
    long long settled = 0;
};
