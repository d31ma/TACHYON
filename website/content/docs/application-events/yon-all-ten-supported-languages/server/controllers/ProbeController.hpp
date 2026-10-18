#pragma once
#include "../models/ProbeReady.hpp"

class [[RequestRoute("/probe")]] [[Public]] ProbeController {
    [[Needs]] ProbeService& probe;
    [[Needs]] PublishService& publish;

public:
    [[GetRoute]]
    ProbeReady getProbe() { return probe.last; }
};
