#pragma once
#include <string>

struct [[Entity({"adapter":"indexeddb"}), Table("products")]] ProductEntity {
    [[Id]] long long id = 0;
    std::string status;
};
