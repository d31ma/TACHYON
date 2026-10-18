namespace tachyon::yon {

class SocketPeer {
public:
    SocketPeer(std::string requestId, std::string peerId) : requestId(std::move(requestId)), peerId(std::move(peerId)) {}

    const std::string& id() const {
        return peerId;
    }

    template <typename T>
    void send(const T& message) const {
        json::JsonNode fields = YonFrames::event(json::JsonSerializer::serializeToNode(message).toJsonString());
        fields["to"] = peerId;
        YonFrames::send(requestId, std::move(fields));
    }

private:
    std::string requestId;
    std::string peerId;
};

class SocketSession {
public:
    explicit SocketSession(std::string id) : sessionId(std::move(id)) {}

    SocketSession(const SocketSession&) = delete;
    SocketSession& operator=(const SocketSession&) = delete;

    const std::string& id() const {
        return sessionId;
    }

    template <typename T>
    void send(const T& message) {
        YonFrames::send(sessionId, YonFrames::event(json::JsonSerializer::serializeToNode(message).toJsonString()));
    }

    void close() {
        closed = true;
        YonFrames::send(sessionId, YonFrames::kind("close"));
    }

    std::vector<SocketPeer> openSessions() {
        sessions.reset();
        YonFrames::send(sessionId, YonFrames::kind("sessions"));
        while (!sessions.has_value() && !closed) {
            receive();
        }
        std::vector<SocketPeer> peers;
        for (const std::string& peer : sessions.value_or(std::vector<std::string>())) {
            peers.emplace_back(sessionId, peer);
        }
        return peers;
    }

private:
    friend class YonSocket;

    std::string sessionId;
    std::deque<json::JsonNode> inbox;
    bool closed = false;
    std::optional<std::vector<std::string>> sessions;

    bool next(json::JsonNode& message) {
        while (inbox.empty() && !closed) {
            receive();
        }
        if (inbox.empty()) {
            return false;
        }
        message = std::move(inbox.front());
        inbox.pop_front();
        return true;
    }

    void receive() {
        std::optional<json::JsonNode> frame = read();
        if (!frame.has_value()) {
            closed = true;
            return;
        }
        if (YonFrames::text(*frame, "request_id") != sessionId) {
            std::exit(YonFrames::PROTOCOL_FAILURE);
        }
        std::string kind = YonFrames::text(*frame, "kind");
        if (kind == "message") {
            inbox.push_back(parsed(YonFrames::text(YonFrames::field(*frame, "body"), "data")));
        } else if (kind == "sessions") {
            sessions = ids(YonFrames::field(*frame, "sessions"));
        } else {
            closed = true;
        }
    }

    static std::optional<json::JsonNode> read() {
        std::string raw;
        if (!YonFrames::read(raw)) {
            return std::nullopt;
        }
        try {
            return json::JsonReader::read(raw, YonFrames::MAX_BYTES);
        } catch (const json::JsonException&) {
            return std::nullopt;
        }
    }

    static json::JsonNode parsed(const std::string& data) {
        try {
            return json::JsonReader::read(data, YonFrames::MAX_BYTES);
        } catch (const json::JsonException&) {
            return json::JsonNode(nullptr);
        }
    }

    static std::vector<std::string> ids(const json::JsonNode& values) {
        std::vector<std::string> found;
        if (values.getValueKind() != json::JsonValueKind::Array) {
            return found;
        }
        for (const json::JsonNode& value : values.items()) {
            if (value.getValueKind() == json::JsonValueKind::String) {
                found.push_back(value.getValue<std::string>());
            }
        }
        return found;
    }
};

class YonSocket {
public:
    template <typename T>
    static T message(const json::JsonNode& value) {
        return json::JsonSerializer::deserialize<T>(value);
    }

    template <typename Open, typename Message, typename Close, typename Failed>
    static YonAnswer run(SocketSession& session, YonInvocation& invocation, Open open, Message message, Close close, Failed failed) {
        attempt(session, invocation, open, failed);
        json::JsonNode value;
        while (session.next(value)) {
            attempt(session, invocation, [&]() { message(value); }, failed);
        }
        call(invocation, close);
        return YonAnswer::streamed();
    }

private:
    template <typename Action>
    static std::optional<std::string> call(YonInvocation& invocation, Action action) {
        try {
            action();
            invocation.drain();
            return std::nullopt;
        } catch (const std::exception& error) {
            return std::string(error.what());
        } catch (...) {
            return std::string("A socket method threw a value that is not a std::exception.");
        }
    }

    template <typename Action, typename Failed>
    static void attempt(SocketSession& session, YonInvocation& invocation, Action action, Failed failed) {
        std::optional<std::string> failure = call(invocation, action);
        if (!failure.has_value()) {
            return;
        }
        json::JsonObject line;
        line["event"] = "handler.socket_failed";
        line["request_id"] = session.id();
        line["message"] = failure->substr(0, 2048);
        std::string written = line.toJsonString() + "\n";
        std::fwrite(written.data(), 1, written.size(), stderr);
        call(invocation, [&]() { failed(*failure); });
    }
};

}

using tachyon::yon::SocketPeer;
using tachyon::yon::SocketSession;
