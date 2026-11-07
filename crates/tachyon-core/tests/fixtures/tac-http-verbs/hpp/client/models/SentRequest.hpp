#pragma once
#include <string>

struct SentRequest {
    std::string method;
    std::string path;
    std::string header;
    std::string item;
};
