extern "C" {
__attribute__((import_module("tachyon"), import_name("tac_host_emit"))) void tac_host_emit(const char* text, int length);
__attribute__((import_module("tachyon"), import_name("tac_host_storage"))) char* tac_host_storage(const char* text, int length);
__attribute__((import_module("tachyon"), import_name("tac_host_web"))) char* tac_host_web(const char* text, int length);
__attribute__((import_module("tachyon"), import_name("tac_host_epoch_ms"))) double tac_host_epoch_ms();
}

namespace tachyon {

class TacHost {
public:
    static void emit(const std::string& text) {
        tac_host_emit(text.data(), static_cast<int>(text.size()));
    }

    static std::string storage(const std::string& request) {
        return answered(tac_host_storage(request.data(), static_cast<int>(request.size())), "Tachyon storage is not on this page.");
    }

    static std::string web(const std::string& request) {
        return answered(tac_host_web(request.data(), static_cast<int>(request.size())), "Tachyon browser APIs are not on this page.");
    }

    static double epochMilliseconds() {
        return tac_host_epoch_ms();
    }

private:
    static std::string answered(char* answer, const char* missing) {
        if (answer == nullptr) {
            json::JsonObject failure;
            failure["error"] = missing;
            return failure.toJsonString();
        }
        std::string text(answer);
        std::free(answer);
        return text;
    }
};

}
