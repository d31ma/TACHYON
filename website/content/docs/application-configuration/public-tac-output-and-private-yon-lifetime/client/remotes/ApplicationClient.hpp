#pragma once

class [[HTTPClient]] ApplicationClient {
public:
    [[GetRequest("/manifest.webmanifest")]]
    Task<tachyon::json::JsonNode> load();
};
