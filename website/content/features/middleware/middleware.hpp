#pragma once
#include <optional>

class Middleware {
public:
    std::optional<tachyon::http::HttpResponseMessage> before(const tachyon::http::HttpRequestMessage& request) {
        bool isPrivate = request.requestUri.rfind("/private", 0) == 0;
        if (!isPrivate || request.headers.get("Authorization") == "let-me-in") {
            return std::nullopt;
        }
        tachyon::http::HttpResponseMessage response;
        response.statusCode = tachyon::http::HttpStatusCode::Unauthorized;
        response.headers.add("content-type", "application/json");
        response.content = tachyon::http::StringContent("{\"error\":\"Unauthorized\"}");
        return response;
    }

    tachyon::http::HttpResponseMessage after(const tachyon::http::HttpRequestMessage& request) {
        tachyon::http::HttpResponseMessage response;
        response.statusCode = tachyon::http::HttpStatusCode::NoContent;
        response.headers.add("x-served-by", "tachyon");
        return response;
    }
};
