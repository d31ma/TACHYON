#pragma once

class [[Schedule({"fixedDelaySeconds":300,"initialDelaySeconds":30})]] Handler {
public:
    void POST() {}
};
