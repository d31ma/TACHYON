#pragma once
#include <string>
#include <vector>
#include "ProblemEntry.hpp"

struct Problem {
    std::string title;
    int status = 0;
    std::vector<ProblemEntry> errors;
};
