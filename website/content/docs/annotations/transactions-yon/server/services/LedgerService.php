<?php
#[Service]
class LedgerService {
    public BehaviorContext $behaviors;
    #[Needs]
    private EntryStore $entries;

    #[OnStart]
    public function open(): void {
        $signal = new BehaviorSignal();
        $this->behaviors = Behaviors::create(LedgerSetup::class, ['partition' => 'ledger', 'signal' => $signal]);
    }

    #[AllOrNothing({"adapter":"ledger"})]
    public function settle(string $status): array {
        $open = $this->entries->findByStatus($status);
        $total = $this->entries->countByStatus($status);
        return ['open' => $open, 'total' => $total];
    }
}
