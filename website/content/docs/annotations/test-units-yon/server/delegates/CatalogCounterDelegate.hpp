#pragma once

class [[Delegate]] CatalogCounterDelegate {
public:
    int value = 21;

    int count() const { return value; }
};
