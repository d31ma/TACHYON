import java.util.ArrayList;
import java.util.List;
import models.PlacedOrder;
import models.SettledOrder;

@Service
class OrdersService {
    ApplicationEventPublisher applicationEventPublisher;
    List<String> log = new ArrayList<>();
    int settled = 0;
    long audited = 0;

    void place(long total) {
        applicationEventPublisher.publishEvent(new PlacedOrder("paid", total));
    }

    @OnEvent({"id":"orders-log"})
    @Order(-10)
    void record(PlacedOrder order) { log.add(order.status()); }

    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    SettledOrder settleLargeOrder(PlacedOrder order) {
        settled += 1;
        return new SettledOrder(order.total());
    }

    @OnEvent
    @InBackground
    void audit(SettledOrder order) { audited = order.total(); }
}
