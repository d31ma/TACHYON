#pragma once

class [[Service]] CatalogService {
    [[Needs]] CatalogCounterHelper& catalogCounterHelper;

public:
    int total() const { return catalogCounterHelper.count() * 2; }
};
