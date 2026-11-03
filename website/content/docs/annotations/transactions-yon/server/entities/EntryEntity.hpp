#pragma once
#include <string>

struct [[Entity({"adapter":"ledger"}), Table("entries")]] EntryEntity {
    [[Id]] long long id = 0;
    std::string status;
};
