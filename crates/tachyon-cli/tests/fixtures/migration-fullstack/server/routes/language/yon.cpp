class Handler {
public:
    static YonJson PUT(const YonJson& request) {
        const YonJson* body = request.get("body");
        return YonJson::object({
            {"language", "cpp"},
            {"action", "echo"},
            {"body", body == nullptr ? YonJson() : *body},
        });
    }

    static YonJson PATCH(const YonJson& request) {
        const YonJson* body = request.get("body");
        const std::string payload = body == nullptr ? "" : body->stringify();
        return YonJson::object({
            {"language", "cpp"},
            {"action", "patch-summary"},
            {"receivedBytes", static_cast<double>(payload.size())},
        });
    }
};
