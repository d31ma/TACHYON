#pragma once

class [[HTTPClient]] ReportClient {
public:
    [[Gateway({"class":"server/services/ReportService.py"})]]
    tachyon::json::JsonNode summarize(const tachyon::json::JsonNode& period) const;

    [[Gateway({"command":["python3","server/helpers/forecast.py"]})]]
    tachyon::json::JsonNode forecast(const tachyon::json::JsonNode& months) const;
};
