#pragma once
#include "../models/Tick.hpp"

class [[RequestRoute("/ticks")]] [[Public]] TicksController {
public:
    [[StreamRoute]]
    tachyon::Generator<Tick> streamTicks() {
        return countdown(3);
    }

private:
    tachyon::Generator<Tick> countdown(int start) {
        for (int level = start; level >= 1; level--) {
            co_yield Tick{"ANVIL-1", level};
        }
    }
};
