<?php
use Models\InventoryRecord;

#[RequestRoute("/inventory")]
#[Public]
class InventoryController {
    #[Needs]
    private ApplicationEventPublisher $applicationEventPublisher;

    #[PostRoute]
    public function postRecord(#[FromBody] InventoryRecord $record): InventoryRecord {
        $this->applicationEventPublisher->publishEvent($record);
        return $record;
    }
}
