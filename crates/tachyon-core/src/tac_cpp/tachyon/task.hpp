namespace tachyon {

class TacCodedError : public std::runtime_error {
public:
    TacCodedError(const std::string& message, std::string code) : std::runtime_error(message), errorCode(std::move(code)) {}

    const std::string& code() const {
        return errorCode;
    }

private:
    std::string errorCode;
};

}

namespace tachyon {

template <typename T>
class Task;

class TacNothing {};

template <typename T>
using TacStored = std::conditional_t<std::is_void_v<T>, TacNothing, T>;

template <typename T>
class TacTaskSlot {
public:
    std::optional<TacStored<T>> value;
    std::exception_ptr failure;
    std::coroutine_handle<> continuation;
    bool done = false;
};

template <typename T>
class TacPromiseCore {
public:
    std::shared_ptr<TacTaskSlot<T>> slot = std::make_shared<TacTaskSlot<T>>();

    std::suspend_never initial_suspend() noexcept {
        return {};
    }

    auto final_suspend() noexcept {
        class Final {
        public:
            std::shared_ptr<TacTaskSlot<T>> slot;

            bool await_ready() noexcept {
                return false;
            }

            std::coroutine_handle<> await_suspend(std::coroutine_handle<>) noexcept {
                std::coroutine_handle<> next = slot->continuation;
                return next ? next : std::noop_coroutine();
            }

            void await_resume() noexcept {}
        };
        return Final{slot};
    }

    void unhandled_exception() {
        slot->failure = std::current_exception();
        slot->done = true;
    }

    Task<T> get_return_object();
};

template <typename T>
class TacValuePromise : public TacPromiseCore<T> {
public:
    void return_value(T value) {
        this->slot->value = std::move(value);
        this->slot->done = true;
    }
};

template <typename T>
class TacVoidPromise : public TacPromiseCore<T> {
public:
    void return_void() {
        this->slot->value = TacNothing{};
        this->slot->done = true;
    }
};

template <typename T>
class Task {
public:
    using promise_type = std::conditional_t<std::is_void_v<T>, TacVoidPromise<T>, TacValuePromise<T>>;

    Task(std::shared_ptr<TacTaskSlot<T>> slot, std::coroutine_handle<promise_type> handle)
        : slot(std::move(slot)), handle(handle) {}

    Task(Task&& other) noexcept : slot(std::move(other.slot)), handle(std::exchange(other.handle, {})) {}

    Task(const Task&) = delete;
    Task& operator=(const Task&) = delete;
    Task& operator=(Task&&) = delete;

    ~Task() {
        if (handle) {
            handle.destroy();
        }
    }

    bool await_ready() const noexcept {
        return slot->done;
    }

    void await_suspend(std::coroutine_handle<> waiting) noexcept {
        slot->continuation = waiting;
    }

    T await_resume() {
        if (slot->failure) {
            std::rethrow_exception(slot->failure);
        }
        if constexpr (!std::is_void_v<T>) {
            return std::move(*slot->value);
        }
    }

private:
    std::shared_ptr<TacTaskSlot<T>> slot;
    std::coroutine_handle<promise_type> handle;
};

template <typename T>
Task<T> TacPromiseCore<T>::get_return_object() {
    using Promise = typename Task<T>::promise_type;
    std::coroutine_handle<Promise> handle = std::coroutine_handle<Promise>::from_promise(static_cast<Promise&>(*this));
    return Task<T>(slot, handle);
}

template <typename T>
class TacIsTask : public std::false_type {};

template <typename T>
class TacIsTask<Task<T>> : public std::true_type {
public:
    using Value = T;
};

}

namespace tachyon {

class TacDetached {
public:
    class promise_type {
    public:
        TacDetached get_return_object() noexcept {
            return {};
        }

        std::suspend_never initial_suspend() noexcept {
            return {};
        }

        std::suspend_never final_suspend() noexcept {
            return {};
        }

        void return_void() noexcept {}

        void unhandled_exception() noexcept {
            std::fputs("A Tachyon C++ task failed after it reported.\n", stderr);
        }
    };
};

}
