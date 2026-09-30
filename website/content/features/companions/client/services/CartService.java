import models.CartTotal;

@Service
class CartService {
    int total = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void add() {
        total += 1;
        applicationEventPublisher.publishEvent(new CartTotal(total), true);
    }
}
