#pragma once

struct CacheReport {
    tachyon::json::JsonNode first;
    tachyon::json::JsonNode repeat;
    tachyon::json::JsonNode written;
    tachyon::json::JsonNode refreshed;
};
