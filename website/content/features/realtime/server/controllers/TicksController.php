<?php
use Models\Tick;

#[RequestRoute("/ticks")]
#[Public]
class TicksController {
    #[StreamRoute]
    public function streamTicks(): Generator {
        yield from $this->ticksFor(['ANVIL-1' => 3, 'ANVIL-2' => 1]);
    }

    private function ticksFor(array $levels): Generator {
        foreach ($levels as $sku => $level) {
            yield new Tick($sku, $level);
        }
    }
}
