#pragma once
#include "../models/Item.hpp"
#include "../models/ItemList.hpp"

class [[RequestRoute("/items")]] [[Public]] ItemsController {
public:
    [[GetRoute]]
    ItemList getItems() {
        return ItemList{};
    }

    [[PostRoute]] [[ResponseStatus(201)]]
    Item postItem([[FromBody]] Item item) {
        return item;
    }
};
