#pragma once
#include <string>

class [[Service]] ActivityService {
public:
    std::string state = "foreground";

    [[OnEvent({"replay":true})]]
    void applicationStateChanged(const ApplicationStateChanged& changed) { state = changed.state; }
};
