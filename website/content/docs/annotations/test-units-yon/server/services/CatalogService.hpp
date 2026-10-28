#pragma once

class [[Service]] CatalogService {
    [[Needs]] CatalogCounterDelegate& catalogCounterDelegate;

public:
    int total() const { return catalogCounterDelegate.count() * 2; }
};
