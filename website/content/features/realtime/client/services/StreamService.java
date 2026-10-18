import java.util.ArrayList;
import java.util.List;
import models.Tick;

@Service
class StreamService {
    private static final int TICK_LIMIT = 128;

    @Needs
    private final TicksClient ticksClient;
    List<Tick> ticks = new ArrayList<>();

    @OnEvent
    void addTick(Tick tick) {
        int older = Math.max(0, ticks.size() - (TICK_LIMIT - 1));
        List<Tick> kept = new ArrayList<>(ticks.subList(older, ticks.size()));
        kept.add(tick);
        ticks = kept;
    }
}
