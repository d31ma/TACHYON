namespace tachyon {

using BehaviorHTTPAdapter = std::function<json::JsonNode(const http::HttpRequestMessage&)>;

class BehaviorAdapters {
public:
    std::map<std::string, BehaviorHTTPAdapter> http;
};

class TacTestAnswers {
public:
    explicit TacTestAnswers(BehaviorAdapters adapters) : previous(current()) {
        current() = std::move(adapters);
    }

    ~TacTestAnswers() {
        current() = std::move(previous);
    }

    TacTestAnswers(const TacTestAnswers&) = delete;
    TacTestAnswers& operator=(const TacTestAnswers&) = delete;

    static http::HttpResponseMessage answer(const std::string& adapter, const http::HttpRequestMessage& request) {
        auto found = current().http.find(adapter);
        if (found == current().http.end()) {
            throw http::HttpRequestException("HTTP_ADAPTER", missing(adapter));
        }
        http::HttpResponseMessage response;
        response.content = http::HttpContent(found->second(request).toJsonString());
        return response;
    }

private:
    BehaviorAdapters previous;

    static BehaviorAdapters& current() {
        static BehaviorAdapters adapters;
        return adapters;
    }

    static std::string missing(const std::string& adapter) {
        return "The C++ test unit has no HTTP answer for the adapter \"" + adapter + "\". Return one in adapters.http[\"" + adapter + "\"] from the static configure() method of the TestSetup class.";
    }
};

}

namespace tachyon::http {

class TacTransport {
public:
    static Task<HttpResponseMessage> send(std::string adapter, HttpRequestMessage request) {
        co_return TacTestAnswers::answer(adapter, request);
    }
};

}

using tachyon::BehaviorAdapters;
using tachyon::BehaviorHTTPAdapter;
