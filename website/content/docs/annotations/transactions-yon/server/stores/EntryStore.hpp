#pragma once
#include <string>
#include <vector>

class [[Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})]] EntryStore {
public:
    std::vector<EntryEntity> findByStatus(const std::string& status);
    long long countByStatus(const std::string& status);
};
