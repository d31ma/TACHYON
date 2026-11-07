#pragma once

class [[HTTPClient]] LedgerClient {
public:
    [[Gateway({"class":"server/services/LedgerService.py"})]]
    tachyon::json::JsonNode balance(const tachyon::json::JsonNode& account) const;

    [[Gateway({"command":["python3","server/helpers/forecast.py"]})]]
    tachyon::json::JsonNode forecast(const tachyon::json::JsonNode& months) const;
};
