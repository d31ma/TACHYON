namespace tachyon {

using BehaviorHTTPAdapter = std::function<json::JsonNode(const http::HttpRequestMessage&)>;

class BehaviorAdapters {
public:
    std::map<std::string, BehaviorHTTPAdapter> http;
};

class BehaviorExecution {
public:
    std::string environment;
    std::string partition;
    std::vector<std::string> profiles;
};

class BehaviorOptions {
public:
    std::string partition;
    std::vector<std::string> profiles;
    std::map<std::string, std::string> properties;

    BehaviorOptions& withApplication(const json::JsonNode& application) {
        properties = TacSettingsBinder::fromApplication(application);
        return *this;
    }
};

template <typename Setup>
struct TacSetupPlan;

class TacSetups {
public:
    template <typename Setup>
    static std::shared_ptr<const BehaviorAdapters> acquire(const std::string& key, const BehaviorExecution& execution) {
        auto found = open().find(key);
        if (found != open().end()) {
            found->second.users += 1;
            return found->second.adapters;
        }
        auto adapters = std::make_shared<const BehaviorAdapters>(configured<Setup>(execution));
        open()[key] = TacSharedSetup{adapters, 1};
        return adapters;
    }

    static void release(const std::string& key) {
        auto found = open().find(key);
        if (found == open().end()) {
            return;
        }
        found->second.users -= 1;
        if (found->second.users == 0) {
            open().erase(found);
        }
    }

private:
    class TacSharedSetup {
    public:
        std::shared_ptr<const BehaviorAdapters> adapters;
        std::size_t users = 0;
    };

    static std::map<std::string, TacSharedSetup>& open() {
        static std::map<std::string, TacSharedSetup> setups;
        return setups;
    }

    template <typename Setup>
    static BehaviorAdapters configured(const BehaviorExecution& execution) {
        if constexpr (requires { Setup::configure(execution); }) {
            return Setup::configure(execution);
        } else if constexpr (requires { Setup::configure(); }) {
            return Setup::configure();
        } else {
            return BehaviorAdapters();
        }
    }
};

class TacSetupSelection {
public:
    static void check(const BehaviorOptions& options) {
        try {
            bool blank = std::all_of(options.partition.begin(), options.partition.end(), [](char character) {
                return std::isspace(static_cast<unsigned char>(character)) != 0;
            });
            if (blank || options.partition.size() > 256 || options.profiles.size() > 32) {
                throw TacBehaviorData::invalid();
            }
            std::vector<std::string> seen;
            for (const std::string& profile : options.profiles) {
                if (!named(profile) || std::find(seen.begin(), seen.end(), profile) != seen.end()) {
                    throw TacBehaviorData::invalid();
                }
                seen.push_back(profile);
            }
            TacSettingsBinder::check(options.properties);
        } catch (const std::exception&) {
            throw TacCodedError("Invalid explicit configuration inputs.", "configuration");
        }
    }

    static void active(const json::JsonNode& plan, const BehaviorOptions& options) {
        bool profiles = !TacBehaviorData::present(plan["profiles"]) || std::any_of(options.profiles.begin(), options.profiles.end(), [&](const std::string& profile) {
            return TacBehaviorData::listed(plan["profiles"], profile);
        });
        if (!profiles || !setting(plan["conditionalOnProperty"], options.properties)) {
            throw TacCodedError("Setup is inactive for the explicit selection.", "inactive-configuration");
        }
    }

    static std::string key(const json::JsonNode& plan, const BehaviorOptions& options) {
        json::JsonArray key;
        key.add(plan["name"]);
        json::JsonArray selected;
        for (const std::string& profile : sorted(options)) {
            selected.add(profile);
        }
        key.add(selected);
        json::JsonObject properties;
        for (const auto& [name, value] : options.properties) {
            properties[name] = value;
        }
        key.add(properties);
        return key.toJsonString();
    }

    static std::vector<std::string> sorted(const BehaviorOptions& options) {
        std::vector<std::string> profiles = options.profiles;
        std::sort(profiles.begin(), profiles.end());
        return profiles;
    }

private:
    static bool named(const std::string& profile) {
        if (profile.empty() || profile.size() > 64 || std::isdigit(static_cast<unsigned char>(profile[0]))) {
            return false;
        }
        return std::all_of(profile.begin(), profile.end(), [](char character) {
            return std::isalnum(static_cast<unsigned char>(character)) || character == '_' || character == '$';
        });
    }

    static bool setting(const json::JsonNode& condition, const std::map<std::string, std::string>& properties) {
        if (!TacBehaviorData::present(condition)) {
            return true;
        }
        std::string name = condition["name"].getValue<std::string>();
        for (const auto& [key, value] : properties) {
            if (key.rfind(name + "_", 0) == 0) {
                return false;
            }
        }
        auto found = properties.find(name);
        if (found == properties.end()) {
            return condition["matchIfMissing"].getValueKind() == json::JsonValueKind::True;
        }
        return found->second == condition["havingValue"].getValue<std::string>();
    }
};

class BehaviorContext {
public:
    BehaviorContext() = default;

    BehaviorContext(std::string key, std::shared_ptr<const BehaviorAdapters> adapters, const BehaviorOptions& options)
        : opened(std::make_shared<TacOpened>(std::move(key), std::move(adapters), options)) {}

    bool isOpen() const {
        return opened != nullptr && !opened->released;
    }

    const std::string& partition() const {
        return state().options.partition;
    }

    const std::vector<std::string>& profiles() const {
        return state().options.profiles;
    }

    const BehaviorAdapters& adapters() const {
        return *state().adapters;
    }

    template <typename Settings>
    Settings properties() const;

    void dispose() {
        if (opened != nullptr) {
            opened->release();
        }
    }

private:
    class TacOpened {
    public:
        TacOpened(std::string key, std::shared_ptr<const BehaviorAdapters> adapters, BehaviorOptions options)
            : key(std::move(key)), adapters(std::move(adapters)), options(std::move(options)) {}

        ~TacOpened() {
            release();
        }

        TacOpened(const TacOpened&) = delete;
        TacOpened& operator=(const TacOpened&) = delete;

        void release() {
            if (!released) {
                released = true;
                TacSetups::release(key);
            }
        }

        std::string key;
        std::shared_ptr<const BehaviorAdapters> adapters;
        BehaviorOptions options;
        bool released = false;
    };

    std::shared_ptr<TacOpened> opened;

    const TacOpened& state() const {
        if (!isOpen()) {
            throw TacCodedError("The behavior context is not open.", "cancelled");
        }
        return *opened;
    }
};

class Behaviors {
public:
    template <typename Setup>
    static BehaviorContext create(const BehaviorOptions& options) {
        const json::JsonNode& plan = TacSetupPlan<Setup>::plan();
        TacSetupSelection::check(options);
        TacSetupSelection::active(plan, options);
        std::string key = TacSetupSelection::key(plan, options);
        BehaviorExecution execution{TacHost::REALM, options.partition, TacSetupSelection::sorted(options)};
        return BehaviorContext(key, TacSetups::acquire<Setup>(key, execution), options);
    }

    template <typename Settings>
    static Settings properties(const std::map<std::string, std::string>& values) {
        return bound<Settings>([&]() { return TacSettingsBinder::properties(TacSettingsPlan<Settings>::plan(), values); }, PROPERTIES);
    }

    template <typename Settings>
    static Settings properties(const json::JsonNode& application) {
        return bound<Settings>([&]() {
            return TacSettingsBinder::properties(TacSettingsPlan<Settings>::plan(), TacSettingsBinder::fromApplication(application));
        }, PROPERTIES);
    }

    template <typename Model>
    static Model values(const json::JsonNode& application) {
        return bound<Model>([&]() { return TacSettingsBinder::values(TacValuesPlan<Model>::plan(), application); }, VALUES);
    }

private:
    static constexpr const char* PROPERTIES = "Invalid or missing explicit configuration properties.";
    static constexpr const char* VALUES = "Invalid or missing application configuration value.";

    template <typename Model, typename Bind>
    static Model bound(Bind bind, const char* message) {
        try {
            return json::JsonCodec<Model>::fromJson(bind());
        } catch (const std::exception&) {
            throw TacCodedError(message, "configuration-properties");
        }
    }
};

template <typename Settings>
Settings BehaviorContext::properties() const {
    return Behaviors::properties<Settings>(state().options.properties);
}

}

using tachyon::BehaviorAdapters;
using tachyon::BehaviorContext;
using tachyon::BehaviorExecution;
using tachyon::BehaviorHTTPAdapter;
using tachyon::BehaviorOptions;
using tachyon::Behaviors;
