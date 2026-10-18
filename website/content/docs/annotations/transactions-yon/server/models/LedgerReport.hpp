#pragma once
#include <string>
#include <vector>

struct LedgerReport {
    int open = 0;
    long long total = 0;
    std::vector<std::string> commands;
};
