#pragma once
#include "../models/InventoryRecord.hpp"

class [[RequestRoute("/inventory")]] [[Public]] InventoryController {
    ApplicationEventPublisher applicationEventPublisher;

public:
    [[PostRoute]]
    InventoryRecord postRecord([[FromBody]] InventoryRecord record) {
        applicationEventPublisher.publishEvent(record);
        return record;
    }
};
