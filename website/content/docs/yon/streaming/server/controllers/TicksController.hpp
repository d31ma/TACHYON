#pragma once
#include "../models/Tick.hpp"

class [[RequestRoute("/ticks")]] [[Public]] TicksController {
public:
    [[StreamRoute]]
    tachyon::Generator<Tick> streamTicks() {
        return ticksUpTo(3);
    }

private:
    tachyon::Generator<Tick> ticksUpTo(int last) {
        for (int level = 1; level <= last; level++) {
            co_yield Tick{"ANVIL-1", level};
        }
    }
};
