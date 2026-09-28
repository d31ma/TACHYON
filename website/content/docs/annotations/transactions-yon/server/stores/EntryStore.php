<?php
#[Store(EntryEntity::class, {"fields":["id","status"],"defaultSize":20,"maxSize":100})]
class EntryStore {
    #[Needs]
    public BehaviorContext $behaviors;
    public function findByStatus(string $status): array {}
    public function countByStatus(string $status): int {}

    #[AllOrNothing({"adapter":"ledger"})]
    public function settle(string $status): array {
        $open = $this->findByStatus($status);
        $total = $this->countByStatus($status);
        return ['open' => $open, 'total' => $total];
    }
}
