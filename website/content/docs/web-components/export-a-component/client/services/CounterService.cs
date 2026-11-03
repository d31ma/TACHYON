using Models;

[Service]
public class CounterService {
    public int Count = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public void Add() {
        Count += 1;
        applicationEventPublisher.PublishEvent(new CountChanged(Count));
    }
}
