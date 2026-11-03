import java.util.stream.IntStream;
import java.util.stream.Stream;
import models.Tick;

@RequestRoute("/ticks")
@Public
class TicksController {
    @StreamRoute
    Stream<Tick> streamTicks() {
        return countdown(3);
    }

    private Stream<Tick> countdown(int start) {
        return IntStream.iterate(start, level -> level > 0, level -> level - 1)
            .mapToObj(level -> new Tick("ANVIL-1", level));
    }
}
