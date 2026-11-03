#pragma once
#include <string>

class [[Service]] StatusService {
public:
    bool online = false;
    std::string scheme;

    [[OnEvent({"replay":true})]]
    void connectivityChanged(const ConnectivityChanged& changed) { online = changed.online; }

    [[OnEvent({"replay":true})]]
    void preferencesChanged(const PreferencesChanged& changed) { scheme = changed.scheme; }
};
