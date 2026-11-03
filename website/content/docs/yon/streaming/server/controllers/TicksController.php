<?php
use Models\Tick;

#[RequestRoute("/ticks")]
#[Public]
class TicksController {
    #[StreamRoute]
    public function streamTicks(): Generator {
        yield from $this->ticksUpTo(3);
    }

    private function ticksUpTo(int $last): Generator {
        for ($level = 1; $level <= $last; $level++) {
            yield new Tick('ANVIL-1', $level);
        }
    }
}
