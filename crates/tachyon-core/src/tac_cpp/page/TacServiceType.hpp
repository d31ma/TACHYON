namespace tachyon {

class TacMember {
public:
    std::string name;
    std::function<json::JsonNode(void*)> read;
    std::function<void(void*, const json::JsonNode&)> write;
};

class TacMethod {
public:
    std::string name;
    std::function<json::JsonNode(void*, const json::JsonNode&)> call;
};

class TacServiceType {
public:
    json::JsonNode metadata;
    std::function<std::shared_ptr<void>()> create;
    std::vector<TacMember> fields;
    std::vector<TacMethod> methods;
    std::vector<TacMethod> hooks;

    const TacMember* field(const std::string& name) const {
        for (const TacMember& member : fields) {
            if (member.name == name) {
                return &member;
            }
        }
        return nullptr;
    }

    const TacMethod* method(const std::string& name) const {
        return find(methods, name);
    }

    const TacMethod* hook(const std::string& name) const {
        return find(hooks, name);
    }

private:
    static const TacMethod* find(const std::vector<TacMethod>& list, const std::string& name) {
        for (const TacMethod& method : list) {
            if (method.name == name) {
                return &method;
            }
        }
        return nullptr;
    }
};

class TacProgram {
public:
    static std::map<std::string, TacServiceType> services();
};

}
