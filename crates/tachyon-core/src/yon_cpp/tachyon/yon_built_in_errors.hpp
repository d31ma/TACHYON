namespace tachyon {

struct BrokenRule {
    std::string field;
    std::string rule;
};

class NotSignedIn : public std::runtime_error {
public:
    static constexpr int status = 401;
    std::string reference;

    explicit NotSignedIn(std::string reference) : std::runtime_error("NotSignedIn"), reference(std::move(reference)) {}
};

class RoleMissing : public std::runtime_error {
public:
    static constexpr int status = 403;
    std::string reference;

    explicit RoleMissing(std::string reference) : std::runtime_error("RoleMissing"), reference(std::move(reference)) {}
};

class IdentityCheckFailed : public std::runtime_error {
public:
    static constexpr int status = 500;
    std::string reference;

    explicit IdentityCheckFailed(std::string reference) : std::runtime_error("IdentityCheckFailed"), reference(std::move(reference)) {}
};

class RequestRejected : public std::runtime_error {
public:
    static constexpr int status = 400;
    std::string reference;
    std::string part;
    std::string field;
    std::string message;

    RequestRejected(std::string reference, std::string part, std::string field, std::string message)
        : std::runtime_error("RequestRejected"), reference(std::move(reference)), part(std::move(part)), field(std::move(field)), message(std::move(message)) {}
};

class BodyTooLarge : public std::runtime_error {
public:
    static constexpr int status = 413;
    std::string reference;
    long long limitBytes;

    BodyTooLarge(std::string reference, long long limitBytes) : std::runtime_error("BodyTooLarge"), reference(std::move(reference)), limitBytes(limitBytes) {}
};

class CheckRejected : public std::runtime_error {
public:
    static constexpr int status = 422;
    std::string reference;
    std::vector<BrokenRule> violations;
    bool truncated;

    CheckRejected(std::string reference, std::vector<BrokenRule> violations, bool truncated)
        : std::runtime_error("CheckRejected"), reference(std::move(reference)), violations(std::move(violations)), truncated(truncated) {}
};

class UnexpectedError : public std::runtime_error {
public:
    static constexpr int status = 500;
    std::string reference;

    explicit UnexpectedError(std::string reference) : std::runtime_error("UnexpectedError"), reference(std::move(reference)) {}
};

class YonBuiltInErrors {
public:
    static std::exception_ptr thrown(const yon::YonFrame& frame) {
        json::JsonNode value = fields(frame.field("arguments"));
        std::string type = text(value, "type");
        std::string reference = text(value, "reference");
        if (type == "NotSignedIn") {
            return std::make_exception_ptr(NotSignedIn(reference));
        }
        if (type == "RoleMissing") {
            return std::make_exception_ptr(RoleMissing(reference));
        }
        if (type == "IdentityCheckFailed") {
            return std::make_exception_ptr(IdentityCheckFailed(reference));
        }
        if (type == "RequestRejected") {
            return std::make_exception_ptr(RequestRejected(reference, text(value, "part"), text(value, "field"), text(value, "message")));
        }
        if (type == "BodyTooLarge") {
            return std::make_exception_ptr(BodyTooLarge(reference, whole(value, "limitBytes")));
        }
        if (type == "CheckRejected") {
            return std::make_exception_ptr(CheckRejected(reference, rules(value), has(value, "truncated", json::JsonValueKind::True)));
        }
        if (type == "UnexpectedError") {
            return std::make_exception_ptr(UnexpectedError(reference));
        }
        throw std::runtime_error("Unknown built-in error.");
    }

private:
    static json::JsonNode fields(const json::JsonNode& arguments) {
        bool listed = arguments.getValueKind() == json::JsonValueKind::Array && arguments.count() > 0;
        if (!listed || arguments[0].getValueKind() != json::JsonValueKind::Object) {
            return json::JsonNode(nullptr);
        }
        return arguments[0];
    }

    static std::string text(const json::JsonNode& value, const std::string& key) {
        if (!has(value, key, json::JsonValueKind::String)) {
            return std::string();
        }
        return value[key].getValue<std::string>();
    }

    static long long whole(const json::JsonNode& value, const std::string& key) {
        if (!has(value, key, json::JsonValueKind::Number)) {
            return 0;
        }
        return static_cast<long long>(value[key].getValue<double>());
    }

    static bool has(const json::JsonNode& value, const std::string& key, json::JsonValueKind kind) {
        return value.getValueKind() == json::JsonValueKind::Object && value.containsKey(key) && value[key].getValueKind() == kind;
    }

    static std::vector<BrokenRule> rules(const json::JsonNode& value) {
        std::vector<BrokenRule> found;
        if (!has(value, "violations", json::JsonValueKind::Array)) {
            return found;
        }
        for (const json::JsonNode& item : value["violations"].items()) {
            found.push_back(BrokenRule{text(item, "field"), text(item, "rule")});
        }
        return found;
    }
};

}

using BrokenRule = tachyon::BrokenRule;
using NotSignedIn = tachyon::NotSignedIn;
using RoleMissing = tachyon::RoleMissing;
using IdentityCheckFailed = tachyon::IdentityCheckFailed;
using RequestRejected = tachyon::RequestRejected;
using BodyTooLarge = tachyon::BodyTooLarge;
using CheckRejected = tachyon::CheckRejected;
using UnexpectedError = tachyon::UnexpectedError;
