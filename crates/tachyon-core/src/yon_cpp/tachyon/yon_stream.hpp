namespace tachyon {

template <typename T>
class Generator {
public:
    class promise_type {
    public:
        std::optional<T> current;
        std::exception_ptr failure;

        Generator get_return_object() {
            return Generator(std::coroutine_handle<promise_type>::from_promise(*this));
        }

        std::suspend_always initial_suspend() noexcept {
            return {};
        }

        std::suspend_always final_suspend() noexcept {
            return {};
        }

        std::suspend_always yield_value(const T& value) {
            current.emplace(value);
            return {};
        }

        std::suspend_always yield_value(T&& value) {
            current.emplace(std::move(value));
            return {};
        }

        void return_void() noexcept {}

        void unhandled_exception() {
            failure = std::current_exception();
        }
    };

    explicit Generator(std::coroutine_handle<promise_type> handle) : handle(handle) {}

    Generator(Generator&& other) noexcept : handle(std::exchange(other.handle, {})) {}

    Generator(const Generator&) = delete;
    Generator& operator=(const Generator&) = delete;
    Generator& operator=(Generator&&) = delete;

    ~Generator() {
        if (handle) {
            handle.destroy();
        }
    }

    bool next() {
        if (!handle || handle.done()) {
            return false;
        }
        handle.promise().current.reset();
        handle.resume();
        if (handle.promise().failure) {
            std::rethrow_exception(handle.promise().failure);
        }
        return !handle.done();
    }

    const T& value() const {
        return *handle.promise().current;
    }

private:
    std::coroutine_handle<promise_type> handle;
};

}

namespace tachyon::yon {

class YonStream {
public:
    static constexpr const char* FAILED_CODE = "TY2204";
    static constexpr const char* FAILED = "The handler stream failed.";
    static constexpr const char* TOO_LARGE_CODE = "TY2203";
    static constexpr const char* TOO_LARGE = "Streamed event exceeds the protocol frame limit.";

    template <typename Make>
    static YonAnswer send(const YonFrame& frame, YonInvocation& invocation, Make make) {
        std::string requestId = frame.text("request_id");
        bool fits = true;
        try {
            auto items = make();
            while (fits && items.next()) {
                fits = sendItem(requestId, items.value());
            }
            invocation.drain();
        } catch (...) {
            YonFrames::send(requestId, YonFrames::failure(FAILED_CODE, FAILED));
            return YonAnswer::streamed();
        }
        if (!fits) {
            YonFrames::send(requestId, YonFrames::failure(TOO_LARGE_CODE, TOO_LARGE));
        }
        return YonAnswer::streamed();
    }

private:
    template <typename T>
    static bool sendItem(const std::string& requestId, const T& item) {
        std::string data = json::JsonSerializer::serializeToNode(item).toJsonString();
        std::string payload = YonFrames::payload(requestId, YonFrames::event(data));
        if (!YonFrames::fits(payload)) {
            return false;
        }
        if (!YonFrames::write(payload)) {
            throw std::runtime_error(FAILED);
        }
        return true;
    }
};

}
