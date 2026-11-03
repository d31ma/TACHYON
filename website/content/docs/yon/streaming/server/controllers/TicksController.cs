using System.Collections.Generic;
using Models;

[RequestRoute("/ticks")]
[Public]
class TicksController {
    [StreamRoute]
    public IEnumerable<Tick> StreamTicks() {
        return TicksUpTo(3);
    }

    private static IEnumerable<Tick> TicksUpTo(int last) {
        for (var level = 1; level <= last; level++) {
            yield return new Tick("ANVIL-1", level);
        }
    }
}
