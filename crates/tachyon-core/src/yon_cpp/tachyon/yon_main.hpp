namespace tachyon::yon {

class YonMain {
public:
    static constexpr std::size_t MAX_FRAME_BYTES = 16 * 1024 * 1024;
    static constexpr int PROTOCOL_FAILURE = 70;
    static constexpr int MAX_FRAME_DEPTH = 256;

    template <typename Routes>
    static int run() {
        std::chrono::steady_clock::time_point started = std::chrono::steady_clock::now();
        std::string raw;
        if (!readInput(raw)) {
            return PROTOCOL_FAILURE;
        }
        std::optional<json::JsonNode> fields = parsed(raw);
        if (!fields.has_value()) {
            return PROTOCOL_FAILURE;
        }
        YonRelay::begin(started, *fields);
        YonDelivery::receive(*fields);
        YonFrame frame(std::move(*fields));
        YonAnswer answer = answered<Routes>(frame);
        if (answer.isStreamed()) {
            return 0;
        }
        if (YonFrames::framed()) {
            return PROTOCOL_FAILURE;
        }
        std::string written = answer.write(YonEvents::current().published());
        writeOutput(written);
        return 0;
    }

private:
    static void writeOutput(const std::string& written) {
        if (YonFrames::enabled()) {
            YonFrames::write(written);
            return;
        }
        std::fwrite(written.data(), 1, written.size(), stdout);
        std::fflush(stdout);
    }

    static bool readInput(std::string& raw) {
        if (YonFrames::enabled() || YonFrames::framed()) {
            return YonFrames::read(raw);
        }
        char buffer[65536];
        std::size_t count = std::fread(buffer, 1, sizeof buffer, stdin);
        while (count > 0) {
            raw.append(buffer, count);
            if (raw.size() > MAX_FRAME_BYTES) {
                return false;
            }
            count = std::fread(buffer, 1, sizeof buffer, stdin);
        }
        return true;
    }

    static std::optional<json::JsonNode> parsed(const std::string& raw) {
        try {
            return json::JsonReader::read(raw, MAX_FRAME_BYTES, MAX_FRAME_DEPTH);
        } catch (const json::JsonException&) {
            return std::nullopt;
        }
    }

    template <typename Routes>
    static YonAnswer answered(const YonFrame& frame) {
        try {
            return Routes::answer(frame);
        } catch (const YonArgumentException& error) {
            return YonAnswer::failure(400, error.what());
        } catch (const TacValidationError& error) {
            return YonAnswer::rejected(error.all());
        } catch (const std::exception& error) {
            report(frame, error.what());
        } catch (...) {
            report(frame, "The route method threw a value that is not a std::exception.");
        }
        return YonAnswer::failure(500, "Handler failed.");
    }

    static void report(const YonFrame& frame, const std::string& message) {
        json::JsonObject line;
        line["event"] = "handler.cpp_exception";
        line["request_id"] = frame.text("request_id");
        line["message"] = message.substr(0, 2048);
        std::string written = line.toJsonString() + "\n";
        std::fwrite(written.data(), 1, written.size(), stderr);
    }
};

}
