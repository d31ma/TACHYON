#include <chrono>
#include <thread>
#ifdef _WIN32
#include <cwchar>
#include <cwctype>
#include <future>
#ifndef NOMINMAX
#define NOMINMAX
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#undef DELETE
#undef IN
#undef OUT
#undef OPTIONAL
#else
#include <cerrno>
#include <csignal>
#include <fcntl.h>
#include <poll.h>
#include <spawn.h>
#include <sys/wait.h>
#include <unistd.h>
extern char** environ;
#endif

namespace tachyon::yon {

class YonRelay {
public:
    static constexpr const char* FAILED = "Component invocation failed.";
    static constexpr std::size_t MAX_STDOUT_BYTES = 16 * 1024 * 1024;
    static constexpr std::size_t MAX_STDERR_BYTES = 64 * 1024;
    static constexpr double DEFAULT_DEADLINE_MILLISECONDS = 30000.0;
    static constexpr double LONGEST_DEADLINE_MILLISECONDS = 86400000.0;
    static constexpr int FAILED_STATUS = 502;

    static void begin(std::chrono::steady_clock::time_point started, const json::JsonNode& fields) {
        const json::JsonNode& written = YonFrames::field(fields, "deadline_ms");
        bool counted = written.getValueKind() == json::JsonValueKind::Number;
        double milliseconds = counted ? written.getValue<double>() : DEFAULT_DEADLINE_MILLISECONDS;
        milliseconds = std::isfinite(milliseconds) ? std::clamp(milliseconds, 0.0, LONGEST_DEADLINE_MILLISECONDS) : DEFAULT_DEADLINE_MILLISECONDS;
        std::string requestId = YonFrames::text(fields, "request_id");
        current().requestId = requestId.empty() ? "unknown" : requestId;
        current().deadline = started + std::chrono::duration_cast<std::chrono::steady_clock::duration>(std::chrono::duration<double, std::milli>(milliseconds));
        current().begun = true;
    }

    static http::HttpResponseMessage send(const std::vector<std::string>& command, const http::HttpRequestMessage& request) {
        if (command.empty() || command[0].empty()) {
            return failure("start");
        }
        if (!current().begun || timeLeft() <= 0) {
            return failure("timeout");
        }
        return exchange(command, message(request));
    }

private:
    struct Invocation {
        std::string requestId = "unknown";
        std::chrono::steady_clock::time_point deadline;
        bool begun = false;
    };

    struct Output {
        std::string kept;
        std::size_t limit;
        bool overflowed = false;
        bool open = true;
    };

    static Invocation& current() {
        static Invocation invocation;
        return invocation;
    }

    static long long timeLeft() {
        auto left = std::chrono::duration_cast<std::chrono::milliseconds>(current().deadline - std::chrono::steady_clock::now());
        return left.count();
    }

    static std::string message(const http::HttpRequestMessage& request) {
        json::JsonObject headers;
        for (const auto& [name, value] : request.headers.all()) {
            json::JsonArray values;
            values.add(value);
            headers[lowered(name)] = values;
        }
        json::JsonObject body;
        body["encoding"] = "utf8";
        body["data"] = request.content.has_value() ? request.content->readAsString() : std::string();
        json::JsonObject written;
        written["route"] = request.requestUri.empty() ? std::string("/") : request.requestUri;
        written["method"] = request.method.toString();
        written["headers"] = headers;
        written["body"] = body;
        return YonText::repaired(written.toJsonString());
    }

    static std::string lowered(std::string name) {
        std::transform(name.begin(), name.end(), name.begin(), [](unsigned char character) {
            return static_cast<char>(std::tolower(character));
        });
        return name;
    }

    static http::HttpResponseMessage failure(const std::string& category) {
        json::JsonObject event;
        event["event"] = "handler.relay_failed";
        event["request_id"] = current().requestId;
        event["category"] = category;
        std::string line = event.toJsonString() + "\n";
        std::fwrite(line.data(), 1, line.size(), stderr);
        std::fflush(stderr);
        json::JsonObject body;
        body["error"] = FAILED;
        http::HttpResponseMessage response;
        response.statusCode = static_cast<http::HttpStatusCode>(FAILED_STATUS);
        response.headers.add("content-type", "application/json");
        response.content = http::HttpContent(body.toJsonString());
        return response;
    }

    static std::optional<http::HttpResponseMessage> envelope(const std::string& written) {
        json::JsonNode answer;
        try {
            answer = json::JsonReader::read(written, MAX_STDOUT_BYTES);
        } catch (const json::JsonException&) {
            return std::nullopt;
        }
        if (answer.getValueKind() != json::JsonValueKind::Object) {
            return std::nullopt;
        }
        const json::JsonNode& status = YonFrames::field(answer, "status");
        int code = status.getValueKind() == json::JsonValueKind::Number ? static_cast<int>(status.getValue<double>()) : 0;
        http::HttpResponseMessage response;
        response.statusCode = static_cast<http::HttpStatusCode>(code == 0 ? 200 : code);
        response.content = http::HttpContent(YonFrames::text(answer, "body"));
        const json::JsonNode& headers = YonFrames::field(answer, "headers");
        for (const std::string& name : headers.propertyNames()) {
            response.headers.add(name, joined(headers[name]));
        }
        return response;
    }

    static std::string joined(const json::JsonNode& values) {
        if (values.getValueKind() == json::JsonValueKind::String) {
            return values.getValue<std::string>();
        }
        std::string text;
        for (const json::JsonNode& value : values.items()) {
            bool written = value.getValueKind() == json::JsonValueKind::String;
            text += written ? (text.empty() ? "" : ", ") + value.getValue<std::string>() : std::string();
        }
        return text;
    }

    static http::HttpResponseMessage answered(bool exitedCleanly, const Output& stdoutText, const Output& stderrText) {
        if (stdoutText.overflowed || stderrText.overflowed) {
            return failure("overflow");
        }
        if (!exitedCleanly) {
            return failure("exit");
        }
        if (!YonText::valid(stdoutText.kept)) {
            return failure("protocol");
        }
        std::optional<http::HttpResponseMessage> response = envelope(stdoutText.kept);
        if (!response.has_value()) {
            return failure("protocol");
        }
        return *response;
    }

#ifdef _WIN32
    struct Pipe {
        HANDLE read = nullptr;
        HANDLE write = nullptr;
    };

    static std::wstring wide(const std::string& text) {
        if (text.empty()) {
            return std::wstring();
        }
        int length = MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), nullptr, 0);
        std::wstring written(static_cast<std::size_t>(std::max(length, 0)), L'\0');
        MultiByteToWideChar(CP_UTF8, 0, text.data(), static_cast<int>(text.size()), written.data(), length);
        return written;
    }

    static std::wstring quoted(const std::wstring& argument) {
        if (!argument.empty() && argument.find_first_of(L" \t\n\v\"") == std::wstring::npos) {
            return argument;
        }
        std::wstring written = L"\"";
        std::size_t backslashes = 0;
        for (wchar_t character : argument) {
            if (character == L'\\') {
                backslashes += 1;
                continue;
            }
            written.append(character == L'"' ? backslashes * 2 + 1 : backslashes, L'\\');
            backslashes = 0;
            written.push_back(character);
        }
        written.append(backslashes * 2, L'\\');
        written.push_back(L'"');
        return written;
    }

    static std::wstring commandLine(const std::vector<std::string>& command) {
        std::wstring written;
        for (const std::string& argument : command) {
            written += (written.empty() ? L"" : L" ") + quoted(wide(argument));
        }
        return written;
    }

    static bool passedOn(const std::wstring& entry) {
        std::wstring name = entry.substr(0, entry.find(L'=', 1));
        std::transform(name.begin(), name.end(), name.begin(), [](wchar_t character) {
            return static_cast<wchar_t>(std::towupper(character));
        });
        return name != L"YON_APPLICATION_JSON" && name.rfind(L"YON_RESOURCE_", 0) != 0;
    }

    static std::wstring environment() {
        std::wstring block;
        wchar_t* strings = GetEnvironmentStringsW();
        for (wchar_t* entry = strings; entry != nullptr && *entry != L'\0'; entry += std::wcslen(entry) + 1) {
            std::wstring pair(entry);
            block += passedOn(pair) ? pair + L'\0' : std::wstring();
        }
        if (strings != nullptr) {
            FreeEnvironmentStringsW(strings);
        }
        block.push_back(L'\0');
        return block;
    }

    static bool openPipe(Pipe& pipe, bool childReads) {
        SECURITY_ATTRIBUTES inherited{sizeof(SECURITY_ATTRIBUTES), nullptr, TRUE};
        if (!CreatePipe(&pipe.read, &pipe.write, &inherited, 0)) {
            return false;
        }
        return SetHandleInformation(childReads ? pipe.write : pipe.read, HANDLE_FLAG_INHERIT, 0) != 0;
    }

    static void closeAll(std::initializer_list<HANDLE> handles) {
        for (HANDLE handle : handles) {
            if (handle != nullptr && handle != INVALID_HANDLE_VALUE) {
                CloseHandle(handle);
            }
        }
    }

    static HANDLE killingJob() {
        HANDLE job = CreateJobObjectW(nullptr, nullptr);
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        bool limited = job != nullptr && SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof limits);
        if (!limited) {
            closeAll({job});
            return nullptr;
        }
        return job;
    }

    static bool spawn(PROCESS_INFORMATION& process, HANDLE job, const std::vector<std::string>& command, const Pipe& input, const Pipe& output, const Pipe& errors) {
        HANDLE inherited[3] = {input.read, output.write, errors.write};
        SIZE_T size = 0;
        InitializeProcThreadAttributeList(nullptr, 1, 0, &size);
        std::vector<unsigned char> storage(size);
        auto attributes = reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
        if (!InitializeProcThreadAttributeList(attributes, 1, 0, &size)) {
            return false;
        }
        bool listed = UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, inherited, sizeof inherited, nullptr, nullptr) != 0;
        STARTUPINFOEXW startup{};
        startup.StartupInfo.cb = sizeof startup;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = input.read;
        startup.StartupInfo.hStdOutput = output.write;
        startup.StartupInfo.hStdError = errors.write;
        startup.lpAttributeList = attributes;
        std::wstring line = commandLine(command);
        std::wstring variables = environment();
        DWORD flags = EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT | CREATE_SUSPENDED | CREATE_NO_WINDOW;
        bool started = listed && CreateProcessW(nullptr, line.data(), nullptr, nullptr, TRUE, flags, variables.data(), nullptr, &startup.StartupInfo, &process) != 0;
        DeleteProcThreadAttributeList(attributes);
        if (!started) {
            return false;
        }
        if (!AssignProcessToJobObject(job, process.hProcess)) {
            TerminateProcess(process.hProcess, 1);
            closeAll({process.hThread, process.hProcess});
            return false;
        }
        ResumeThread(process.hThread);
        return true;
    }

    static void deliverAll(HANDLE end, const std::string& payload) {
        std::size_t sent = 0;
        DWORD count = 0;
        while (sent < payload.size() && WriteFile(end, payload.data() + sent, static_cast<DWORD>(std::min<std::size_t>(payload.size() - sent, 65536)), &count, nullptr)) {
            sent += count;
        }
        CloseHandle(end);
    }

    static void collectAll(HANDLE end, Output& output) {
        char chunk[8192];
        DWORD count = 0;
        while (ReadFile(end, chunk, sizeof chunk, &count, nullptr) && count > 0) {
            std::size_t room = output.limit - output.kept.size();
            std::size_t taken = std::min<std::size_t>(room, count);
            output.kept.append(chunk, taken);
            output.overflowed = output.overflowed || count > room;
        }
        output.open = false;
        CloseHandle(end);
    }

    static bool finishedInTime(HANDLE process, std::initializer_list<std::future<void>*> transfers) {
        for (std::future<void>* transfer : transfers) {
            if (transfer->wait_until(current().deadline) != std::future_status::ready) {
                return false;
            }
        }
        long long left = std::max<long long>(timeLeft(), 0);
        return WaitForSingleObject(process, static_cast<DWORD>(std::min<long long>(left, INFINITE - 1))) == WAIT_OBJECT_0;
    }

    static http::HttpResponseMessage exchange(const std::vector<std::string>& command, const std::string& payload) {
        Pipe input;
        Pipe output;
        Pipe errors;
        HANDLE job = killingJob();
        PROCESS_INFORMATION process{};
        bool opened = job != nullptr && openPipe(input, true) && openPipe(output, false) && openPipe(errors, false);
        bool started = opened && spawn(process, job, command, input, output, errors);
        closeAll({input.read, output.write, errors.write});
        if (!started) {
            closeAll({input.write, output.read, errors.read, job});
            return failure("start");
        }
        Output stdoutText{std::string(), MAX_STDOUT_BYTES};
        Output stderrText{std::string(), MAX_STDERR_BYTES};
        std::future<void> writing = std::async(std::launch::async, deliverAll, input.write, std::cref(payload));
        std::future<void> reading = std::async(std::launch::async, collectAll, output.read, std::ref(stdoutText));
        std::future<void> readingErrors = std::async(std::launch::async, collectAll, errors.read, std::ref(stderrText));
        bool finished = finishedInTime(process.hProcess, {&reading, &readingErrors});
        if (!finished) {
            TerminateJobObject(job, 1);
        }
        writing.wait();
        reading.wait();
        readingErrors.wait();
        DWORD code = 1;
        bool exitedCleanly = finished && GetExitCodeProcess(process.hProcess, &code) && code == 0;
        closeAll({process.hThread, process.hProcess, job});
        if (!finished) {
            return failure("timeout");
        }
        return answered(exitedCleanly, stdoutText, stderrText);
    }
#else
    static std::vector<std::string> environment() {
        std::vector<std::string> kept;
        for (char** entry = environ; entry != nullptr && *entry != nullptr; entry += 1) {
            std::string pair(*entry);
            std::string name = pair.substr(0, pair.find('='));
            std::transform(name.begin(), name.end(), name.begin(), [](unsigned char character) {
                return static_cast<char>(std::toupper(character));
            });
            if (name == "YON_APPLICATION_JSON" || name.rfind("YON_RESOURCE_", 0) == 0) {
                continue;
            }
            kept.push_back(pair);
        }
        return kept;
    }

    static std::vector<char*> pointers(std::vector<std::string>& texts) {
        std::vector<char*> written;
        for (std::string& text : texts) {
            written.push_back(text.data());
        }
        written.push_back(nullptr);
        return written;
    }

    static bool openPipe(int ends[2]) {
        if (pipe(ends) != 0) {
            return false;
        }
        fcntl(ends[0], F_SETFD, FD_CLOEXEC);
        fcntl(ends[1], F_SETFD, FD_CLOEXEC);
        return true;
    }

    static void closeAll(std::initializer_list<int> ends) {
        for (int end : ends) {
            if (end >= 0) {
                close(end);
            }
        }
    }

    static bool spawn(pid_t& child, std::vector<std::string> command, int input[2], int output[2], int errors[2]) {
        posix_spawn_file_actions_t actions;
        posix_spawnattr_t attributes;
        posix_spawn_file_actions_init(&actions);
        posix_spawnattr_init(&attributes);
        posix_spawn_file_actions_adddup2(&actions, input[0], 0);
        posix_spawn_file_actions_adddup2(&actions, output[1], 1);
        posix_spawn_file_actions_adddup2(&actions, errors[1], 2);
        short flags = POSIX_SPAWN_SETSIGDEF;
#if defined(__APPLE__)
        flags |= POSIX_SPAWN_CLOEXEC_DEFAULT;
#elif defined(__GLIBC__) && (__GLIBC__ > 2 || (__GLIBC__ == 2 && __GLIBC_MINOR__ >= 34))
        posix_spawn_file_actions_addclosefrom_np(&actions, 3);
#endif
        sigset_t defaults;
        sigemptyset(&defaults);
        sigaddset(&defaults, SIGPIPE);
        posix_spawnattr_setsigdefault(&attributes, &defaults);
        posix_spawnattr_setflags(&attributes, flags);
        std::vector<std::string> pairs = environment();
        std::vector<char*> arguments = pointers(command);
        std::vector<char*> variables = pointers(pairs);
        int started = posix_spawnp(&child, arguments[0], &actions, &attributes, arguments.data(), variables.data());
        posix_spawn_file_actions_destroy(&actions);
        posix_spawnattr_destroy(&attributes);
        return started == 0;
    }

    static void collect(int& end, Output& output) {
        char chunk[8192];
        ssize_t count = read(end, chunk, sizeof chunk);
        if (count < 0 && (errno == EINTR || errno == EAGAIN)) {
            return;
        }
        if (count <= 0) {
            close(end);
            end = -1;
            output.open = false;
            return;
        }
        std::size_t room = output.limit - output.kept.size();
        std::size_t taken = std::min(room, static_cast<std::size_t>(count));
        output.kept.append(chunk, taken);
        output.overflowed = output.overflowed || static_cast<std::size_t>(count) > room;
    }

    static void deliver(int& end, const std::string& payload, std::size_t& sent) {
        ssize_t count = write(end, payload.data() + sent, payload.size() - sent);
        if (count < 0 && (errno == EINTR || errno == EAGAIN)) {
            return;
        }
        sent += count > 0 ? static_cast<std::size_t>(count) : 0;
        if (count < 0 || sent == payload.size()) {
            close(end);
            end = -1;
        }
    }

    static bool drain(int input, int output, int errors, const std::string& payload, Output& stdoutText, Output& stderrText) {
        std::size_t sent = 0;
        int ends[3] = {input, output, errors};
        if (payload.empty()) {
            close(ends[0]);
            ends[0] = -1;
        }
        while (ends[0] >= 0 || ends[1] >= 0 || ends[2] >= 0) {
            long long left = timeLeft();
            if (left <= 0) {
                closeAll({ends[0], ends[1], ends[2]});
                return false;
            }
            pollfd watched[3] = {
                {ends[0], POLLOUT, 0},
                {ends[1], POLLIN, 0},
                {ends[2], POLLIN, 0},
            };
            int ready = poll(watched, 3, static_cast<int>(std::min<long long>(left, 1000)));
            if (ready < 0 && errno != EINTR) {
                closeAll({ends[0], ends[1], ends[2]});
                return false;
            }
            if (ready <= 0) {
                continue;
            }
            if (ends[0] >= 0 && watched[0].revents != 0) {
                deliver(ends[0], payload, sent);
            }
            if (ends[1] >= 0 && watched[1].revents != 0) {
                collect(ends[1], stdoutText);
            }
            if (ends[2] >= 0 && watched[2].revents != 0) {
                collect(ends[2], stderrText);
            }
        }
        return true;
    }

    static std::optional<int> finish(pid_t child) {
        int status = 0;
        while (true) {
            pid_t done = waitpid(child, &status, WNOHANG);
            if (done == child) {
                return status;
            }
            if (done < 0 && errno != EINTR) {
                return std::nullopt;
            }
            if (timeLeft() <= 0) {
                return std::nullopt;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(2));
        }
    }

    static void stop(pid_t child) {
        kill(child, SIGKILL);
        int status = 0;
        waitpid(child, &status, 0);
    }

    static http::HttpResponseMessage exchange(const std::vector<std::string>& command, const std::string& payload) {
        int input[2] = {-1, -1};
        int output[2] = {-1, -1};
        int errors[2] = {-1, -1};
        if (!openPipe(input) || !openPipe(output) || !openPipe(errors)) {
            closeAll({input[0], input[1], output[0], output[1], errors[0], errors[1]});
            return failure("start");
        }
        fcntl(input[1], F_SETFL, fcntl(input[1], F_GETFL) | O_NONBLOCK);
        struct sigaction ignored {};
        struct sigaction previous {};
        ignored.sa_handler = SIG_IGN;
        sigaction(SIGPIPE, &ignored, &previous);
        pid_t child = -1;
        bool started = spawn(child, command, input, output, errors);
        closeAll({input[0], output[1], errors[1]});
        if (!started) {
            closeAll({input[1], output[0], errors[0]});
            sigaction(SIGPIPE, &previous, nullptr);
            return failure("start");
        }
        Output stdoutText{std::string(), MAX_STDOUT_BYTES};
        Output stderrText{std::string(), MAX_STDERR_BYTES};
        bool drained = drain(input[1], output[0], errors[0], payload, stdoutText, stderrText);
        std::optional<int> status = drained ? finish(child) : std::nullopt;
        sigaction(SIGPIPE, &previous, nullptr);
        if (!status.has_value()) {
            stop(child);
            return failure("timeout");
        }
        return answered(WIFEXITED(*status) && WEXITSTATUS(*status) == 0, stdoutText, stderrText);
    }
#endif
};

}

inline tachyon::http::HttpResponseMessage relay(const std::vector<std::string>& command, const tachyon::http::HttpRequestMessage& request) {
    return tachyon::yon::YonRelay::send(command, request);
}
