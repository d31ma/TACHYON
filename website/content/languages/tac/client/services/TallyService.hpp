#pragma once
#include <string>

class [[Service]] TallyService {
public:
    int count = 6;
    std::string runtime = "C++, compiled by Emscripten";

    int doubled() const {
        return count * 2;
    }
};
