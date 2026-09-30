using Models;

[Service]
public sealed class CartService {
    public int Total = 0;
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public void Add() {
        Total += 1;
        applicationEventPublisher.PublishEvent(new CartTotal(Total), true);
    }
}
