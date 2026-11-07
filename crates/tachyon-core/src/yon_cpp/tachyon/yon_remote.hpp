namespace tachyon::yon {

class YonRemote {
public:
    template <typename Setup>
    static BehaviorContext open(const std::string& partition) {
        BehaviorOptions options;
        options.partition = partition;
        return Behaviors::create<Setup>(options);
    }

    static const BehaviorContext*& answering() {
        static const BehaviorContext* context = nullptr;
        return context;
    }

    template <typename T>
    static T answer(const BehaviorContext& behaviors, const std::string& adapter, const http::HttpRequestMessage& request) {
        const BehaviorContext& used = answering() == nullptr ? behaviors : *answering();
        const std::map<std::string, BehaviorHTTPAdapter>& http = used.adapters().http;
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
        std::string owner = answering() == nullptr ? "its [[Setup]] class" : "the [[TestSetup]] class of the test unit";
        return "The Yon C++ [[HTTPClient]] has no HTTP adapter \"" + adapter + "\". Return one in adapters.http[\"" + adapter + "\"] from the static configure method of " + owner + ".";
    }
};

}
