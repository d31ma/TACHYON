#pragma once
#include <string>
#include "../models/Guide.hpp"

class [[Service]] GuideService {
public:
    std::string title;
    int pages = 0;

    [[OnStart]]
    void start() {
        Guide guide{"Models hold types", 3};
        title = guide.title;
        pages = guide.pages;
    }
};
