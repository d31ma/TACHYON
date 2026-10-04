import models.CountChanged;

@Service
class CounterService {
    int count = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void add() {
        count += 1;
        applicationEventPublisher.publishEvent(new CountChanged(count));
    }
}
