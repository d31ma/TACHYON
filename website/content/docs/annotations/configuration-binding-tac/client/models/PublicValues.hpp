#pragma once
#include <string>

struct PublicValues {
    [[Setting("${tac.catalog.title:Catalog}")]] std::string title;
};
