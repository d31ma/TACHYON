namespace tachyon {

class TacTestAnswers {
public:
    explicit TacTestAnswers(BehaviorContext context) : previous(current()) {
        current() = std::move(context);
    }

    ~TacTestAnswers() {
        current().dispose();
        current() = std::move(previous);
    }

    TacTestAnswers(const TacTestAnswers&) = delete;
    TacTestAnswers& operator=(const TacTestAnswers&) = delete;

    const BehaviorContext& context() const {
        return current();
    }

    static http::HttpResponseMessage answer(const std::string& adapter, const http::HttpRequestMessage& request) {
        const std::map<std::string, BehaviorHTTPAdapter>& http = current().adapters().http;
        auto found = http.find(adapter);
        if (found == http.end()) {
            throw http::HttpRequestException("HTTP_ADAPTER", missing(adapter));
        }
        http::HttpResponseMessage response;
        response.content = http::HttpContent(found->second(request).toJsonString());
        return response;
    }

private:
    BehaviorContext previous;

    static BehaviorContext& current() {
        static BehaviorContext context;
        return context;
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
