#pragma once
#include <string>

class [[RequestRoute("/status")]] [[Public]] StatusController {
public:
    [[GetRoute]]
    std::string getStatus() {
        return "ok";
    }
};
