namespace tachyon {

class TacCall {
public:
    bool publicRead = false;
    bool revalidate = false;
};

}

namespace tachyon::http {

class HttpMethod {
public:
    static const HttpMethod Get;
    static const HttpMethod Post;
    static const HttpMethod Put;
    static const HttpMethod Patch;
    static const HttpMethod Delete;
    static const HttpMethod Head;

    explicit HttpMethod(std::string method) : method(std::move(method)) {}

    const std::string& toString() const {
        return method;
    }

    bool operator==(const HttpMethod& other) const {
        return method == other.method;
    }

private:
    std::string method;
};

inline const HttpMethod HttpMethod::Get{"GET"};
inline const HttpMethod HttpMethod::Post{"POST"};
inline const HttpMethod HttpMethod::Put{"PUT"};
inline const HttpMethod HttpMethod::Patch{"PATCH"};
inline const HttpMethod HttpMethod::Delete{"DELETE"};
inline const HttpMethod HttpMethod::Head{"HEAD"};

}

namespace tachyon::http {

enum class HttpStatusCode : int {
    OK = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    Conflict = 409,
    UnprocessableEntity = 422,
    InternalServerError = 500,
};

}

namespace tachyon::http {

class HttpRequestException : public TacCodedError {
public:
    explicit HttpRequestException(const std::string& code)
        : TacCodedError("The browser HTTP request failed: " + code + ".", code) {}

    HttpRequestException(const std::string& code, const std::string& message) : TacCodedError(message, code) {}
};

}

namespace tachyon::http {

class HttpContent {
public:
    HttpContent() = default;

    explicit HttpContent(std::string json) : json(std::move(json)) {}

    std::string readAsString() const {
        return json;
    }

    template <typename T>
    T readFromJson() const {
        return json::JsonSerializer::deserialize<T>(json);
    }

private:
    std::string json = "null";
};

class StringContent : public HttpContent {
public:
    explicit StringContent(std::string json) : HttpContent(std::move(json)) {}
};

class JsonContent {
public:
    template <typename T>
    static HttpContent create(const T& value) {
        return HttpContent(json::JsonSerializer::serialize(value));
    }
};

}

namespace tachyon::http {

class HttpHeaders {
public:
    void add(const std::string& name, const std::string& value) {
        values[name] = value;
    }

    bool contains(const std::string& name) const {
        return values.count(name) > 0;
    }

    std::optional<std::string> get(const std::string& name) const {
        for (const auto& [written, value] : values) {
            if (sameName(written, name)) {
                return value;
            }
        }
        return std::nullopt;
    }

    const std::map<std::string, std::string>& all() const {
        return values;
    }

private:
    std::map<std::string, std::string> values;

    static bool sameName(const std::string& left, const std::string& right) {
        return std::equal(left.begin(), left.end(), right.begin(), right.end(), [](char first, char second) {
            return std::tolower(static_cast<unsigned char>(first)) == std::tolower(static_cast<unsigned char>(second));
        });
    }
};

}

namespace tachyon::http {

class HttpRequestMessage {
public:
    HttpMethod method;
    std::string requestUri;
    HttpHeaders headers;
    std::optional<HttpContent> content;

    HttpRequestMessage(HttpMethod method, std::string requestUri)
        : method(std::move(method)), requestUri(std::move(requestUri)) {}
};

class HttpResponseMessage {
public:
    HttpStatusCode statusCode = HttpStatusCode::OK;
    std::string reasonPhrase = "OK";
    HttpHeaders headers;
    HttpContent content;

    bool isSuccessStatusCode() const {
        int code = static_cast<int>(statusCode);
        return code >= 200 && code < 300;
    }

    HttpResponseMessage& ensureSuccessStatusCode() {
        if (!isSuccessStatusCode()) {
            throw HttpRequestException("HTTP_STATUS");
        }
        return *this;
    }
};

}
namespace tachyon::http {

class TacBrowserRequest {
public:
    static constexpr double DEADLINE_MILLISECONDS = 30000;
    static constexpr int MOST_RESPONSE_BYTES = 1048576;

    static std::string text(const HttpRequestMessage& request, const TacCall& call = TacCall()) {
        std::size_t mark = request.requestUri.find('?');
        json::JsonObject sent;
        sent["method"] = request.method.toString();
        sent["path"] = request.requestUri.substr(0, mark);
        sent["query"] = query(mark == std::string::npos ? std::string() : request.requestUri.substr(mark + 1));
        sent["headers"] = headers(request.headers);
        sent["redirect"] = "error";
        sent["credentials"] = "omit";
        sent["deadline"] = std::floor(TacHost::epochMilliseconds() + DEADLINE_MILLISECONDS);
        sent["maxResponseBytes"] = MOST_RESPONSE_BYTES;
        sent["publicRead"] = call.publicRead;
        sent["revalidate"] = call.revalidate;
        if (request.content.has_value()) {
            sent["body"] = json::JsonNode::parse(request.content->readAsString());
        }
        return sent.toJsonString();
    }

    static HttpResponseMessage response(const std::string& answer) {
        json::JsonNode settled = json::JsonNode::parse(answer);
        if (settled.containsKey("error")) {
            const json::JsonNode& error = settled["error"];
            bool named = error.getValueKind() == json::JsonValueKind::Object && error.containsKey("code");
            throw HttpRequestException(named ? error["code"].getValue<std::string>() : "HTTP_NETWORK");
        }
        HttpResponseMessage response;
        response.content = HttpContent(settled["value"].toJsonString());
        return response;
    }

    static std::string segment(const std::string& value) {
        bool unsafe = value.empty() || value.find_first_of("/?#% \t\\") != std::string::npos || value == "." || value == "..";
        if (unsafe) {
            throw HttpRequestException("HTTP_ORIGIN");
        }
        return value;
    }

    template <typename T>
    static std::string segment(const T& value) {
        return segment(text(value));
    }

    template <typename T>
    static std::string text(const T& value) {
        json::JsonNode node = json::JsonSerializer::serializeToNode(value);
        bool written = node.getValueKind() == json::JsonValueKind::String;
        return written ? node.getValue<std::string>() : node.toJsonString();
    }

    static std::string text(const char* value) {
        return std::string(value);
    }

    template <typename T>
    static void addQuery(HttpRequestMessage& request, const std::string& key, const T& value) {
        bool first = request.requestUri.find('?') == std::string::npos;
        request.requestUri += (first ? "?" : "&") + key + "=" + text(value);
    }

private:
    static json::JsonNode query(const std::string& written) {
        json::JsonObject values;
        std::size_t start = 0;
        while (start < written.size()) {
            std::size_t end = written.find('&', start);
            std::string pair = written.substr(start, end == std::string::npos ? std::string::npos : end - start);
            std::size_t equals = pair.find('=');
            values[pair.substr(0, equals)] = equals == std::string::npos ? std::string() : pair.substr(equals + 1);
            start = end == std::string::npos ? written.size() : end + 1;
        }
        return values;
    }

    static json::JsonNode headers(const HttpHeaders& written) {
        json::JsonObject values;
        for (const auto& [name, value] : written.all()) {
            values[name] = value;
        }
        return values;
    }
};

}

