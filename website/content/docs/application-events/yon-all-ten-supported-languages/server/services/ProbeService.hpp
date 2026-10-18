#pragma once

class [[Service]] ProbeService {
public:
    ProbeReady last;

private:
    [[OnEvent]]
    void observe(const ProbeReady& ready) { last = ready; }

    [[OnStop]]
    void destroy() { last = ProbeReady{}; }
};
