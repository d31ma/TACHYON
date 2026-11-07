namespace tachyon::http {

class TacRemoteCall {
public:
    static constexpr const char* PLAIN_ADAPTER = "http";

    template <typename T>
    static Task<T> answer(const BehaviorContext* behaviors, std::string adapter, HttpRequestMessage request, TacCall call) {
        HttpResponseMessage response = co_await TacTransport::send(behaviors, std::move(adapter), std::move(request), call);
        if constexpr (std::is_void_v<T>) {
            co_return;
        } else {
            co_return response.content.readFromJson<T>();
        }
    }
};

class HttpClient {
public:
    Task<HttpResponseMessage> sendAsync(HttpRequestMessage request) const {
        return TacTransport::send(TacRemoteCall::PLAIN_ADAPTER, std::move(request));
    }

    Task<HttpResponseMessage> getAsync(std::string requestUri) const {
        return TacTransport::send(TacRemoteCall::PLAIN_ADAPTER, HttpRequestMessage(HttpMethod::Get, std::move(requestUri)));
    }

    template <typename T>
    Task<T> getFromJsonAsync(std::string requestUri) const {
        return TacRemoteCall::answer<T>(nullptr, TacRemoteCall::PLAIN_ADAPTER, HttpRequestMessage(HttpMethod::Get, std::move(requestUri)), TacCall());
    }

    template <typename T>
    Task<HttpResponseMessage> postAsJsonAsync(std::string requestUri, const T& value) const {
        HttpRequestMessage request(HttpMethod::Post, std::move(requestUri));
        request.content = JsonContent::create(value);
        return TacTransport::send(TacRemoteCall::PLAIN_ADAPTER, std::move(request));
    }
};

}
