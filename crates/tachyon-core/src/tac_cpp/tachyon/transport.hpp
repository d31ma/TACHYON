namespace tachyon::http {

class TacTransport {
public:
    static Task<HttpResponseMessage> send(std::string adapter, HttpRequestMessage request) {
        return send(nullptr, std::move(adapter), std::move(request), TacCall());
    }

    static Task<HttpResponseMessage> send(const BehaviorContext* behaviors, std::string adapter, HttpRequestMessage request, TacCall call) {
        const BehaviorHTTPAdapter* adapted = TacAdapters::http(behaviors, adapter);
        if (adapted != nullptr) {
            co_return TacAdapters::answered((*adapted)(request));
        }
        std::string answer = co_await TacHttpTicket(TacBrowserRequest::text(request, call));
        co_return TacBrowserRequest::response(answer);
    }
};

}

namespace tachyon::http {

class TacSocket {
public:
    template <typename Message>
    static Task<void> send(std::string client, Message message) {
        std::string text = json::JsonSerializer::serialize(message);
        if (text.size() > MOST_MESSAGE_BYTES) {
            throw HttpRequestException("HTTP_BODY", "A socket message is JSON of at most 64 KiB.");
        }
        std::string owner = TacTaskOwner::current();
        if (owner.empty()) {
            throw HttpRequestException("HTTP_REQUEST", NOT_SENT);
        }
        json::JsonObject request;
        request["client"] = client;
        request["message"] = json::JsonNode::parse(text);
        answered(co_await TacHttpTicket(owner, request.toJsonString()));
    }

private:
    static constexpr std::size_t MOST_MESSAGE_BYTES = 65536;
    static constexpr const char* NOT_SENT = "The socket message was not sent.";

    static void answered(const std::string& answer) {
        json::JsonNode settled = json::JsonNode::parse(answer);
        if (!settled.containsKey("error") && settled.containsKey("value")) {
            return;
        }
        throw HttpRequestException(code(settled["error"]), NOT_SENT);
    }

    static std::string code(const json::JsonNode& error) {
        bool named = error.getValueKind() == json::JsonValueKind::Object && error.containsKey("code") && error["code"].getValueKind() == json::JsonValueKind::String;
        std::string written = named ? error["code"].getValue<std::string>() : std::string();
        bool known = written == "HTTP_MISSING" || written == "HTTP_REQUEST" || written == "CANCELLED";
        return known ? written : "HTTP_NETWORK";
    }
};

}
