#pragma once

class [[HTTPClient("/billing")]] BillingClient {
public:
    [[GetRequest("/surcharge")]]
    tachyon::json::JsonNode surcharge() const;
};
