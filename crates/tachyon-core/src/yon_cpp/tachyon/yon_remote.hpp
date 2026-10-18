namespace tachyon::yon {

class YonRemote {
public:
    template <typename Setup>
    static BehaviorContext open(const std::string& partition) {
        BehaviorOptions options;
        options.partition = partition;
        return Behaviors::create<Setup>(options);
    }

    template <typename T>
    static T answer(const BehaviorContext& behaviors, const std::string& adapter, const http::HttpRequestMessage& request) {
        const std::map<std::string, BehaviorHTTPAdapter>& http = behaviors.adapters().http;
        auto found = http.find(adapter);
        if (found == http.end()) {
            throw http::HttpRequestException("HTTP_ADAPTER", missing(adapter));
        }
        http::HttpContent answered(found->second(request).toJsonString());
        if constexpr (!std::is_void_v<T>) {
            return answered.readFromJson<T>();
        }
    }

private:
    static std::string missing(const std::string& adapter) {
        return "The Yon C++ [[HTTPClient]] has no HTTP adapter \"" + adapter + "\". Return one in adapters.http[\"" + adapter + "\"] from the static configure method of its [[Setup]] class.";
    }
};

}
