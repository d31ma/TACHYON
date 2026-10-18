#pragma once
#include <string>
#include "../models/Item.hpp"

class [[RequestRoute("/items")]] [[Public]] ItemsController {
public:
    [[GetRoute("/{id}")]]
    Item getItem([[FromPath]] std::string id) {
        return Item{id};
    }

    [[PostRoute]] [[ResponseStatus(201)]]
    Item postItem([[FromBody]] Item item) {
        return item;
    }
};
