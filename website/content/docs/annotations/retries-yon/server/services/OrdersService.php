<?php
#[Service]
class OrdersService {
    public BehaviorContext $behaviors;
    private const FIRST_WORKING_ATTEMPT = 3;
    private int $attempts = 0;

    #[OnStart]
    public function open(): void {
        $signal = new BehaviorSignal();
        $this->behaviors = Behaviors::create(OrdersSetup::class, ['partition' => 'orders', 'signal' => $signal]);
    }

    #[Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]
    public function submit(string $id): array {
        $this->attempts++;
        $this->failFirstTwoAttempts();
        return $this->reportOf($id);
    }

    #[MaxAtOnce({"limit":2,"policy":"reject"})]
    public function report(string $id): array { return $this->reportOf($id); }

    private function reportOf(string $id): array { return ['id' => $id, 'attempts' => $this->attempts]; }

    private function failFirstTwoAttempts(): void {
        if ($this->attempts < self::FIRST_WORKING_ATTEMPT) throw new Exception('the payment gateway is unavailable');
    }
}
