namespace tachyon {

class TacTicket {
public:
    std::optional<std::string> answer;
    std::coroutine_handle<> waiting;
    std::string owner;
};

class TacTickets {
public:
    static std::uint32_t reserve(std::coroutine_handle<> waiting) {
        TacTickets& tickets = current();
        tickets.last = std::max<std::uint32_t>(tickets.last + 1, 1);
        std::uint32_t ticket = tickets.last;
        tickets.waiting[ticket] = TacTicket{std::nullopt, waiting, TacTaskOwner::current()};
        return ticket;
    }

    static void send(std::uint32_t ticket, const std::string& request) {
        TacHost::httpStart(ticket, request);
    }

    static void cancel(std::uint32_t ticket) {
        if (current().waiting.erase(ticket) > 0) {
            TacHost::httpCancel(ticket);
        }
    }

    static void settle(std::uint32_t ticket, std::string answer) {
        TacTickets& tickets = current();
        auto found = tickets.waiting.find(ticket);
        if (found == tickets.waiting.end()) {
            return;
        }
        found->second.answer = std::move(answer);
        std::coroutine_handle<> waiting = found->second.waiting;
        TacTaskOwner::resumeFor(found->second.owner, waiting);
    }

    static std::string take(std::uint32_t ticket) {
        TacTickets& tickets = current();
        auto found = tickets.waiting.find(ticket);
        if (found == tickets.waiting.end()) {
            return LOST;
        }
        std::string answer = found->second.answer.value_or(LOST);
        tickets.waiting.erase(found);
        return answer;
    }

private:
    static constexpr const char* LOST = "{\"error\":{\"code\":\"HTTP_MISSING\"}}";

    std::map<std::uint32_t, TacTicket> waiting;
    std::uint32_t last = 0;

    static TacTickets& current() {
        static TacTickets tickets;
        return tickets;
    }
};

class TacHttpTicket {
public:
    explicit TacHttpTicket(std::string request) : request(std::move(request)) {}

    bool await_ready() const noexcept {
        return false;
    }

    void await_suspend(std::coroutine_handle<> waiting) {
        ticket = TacTickets::reserve(waiting);
        TacTickets::send(ticket, request);
    }

    std::string await_resume() {
        settled = true;
        return TacTickets::take(ticket);
    }

    TacHttpTicket(const TacHttpTicket&) = delete;
    TacHttpTicket& operator=(const TacHttpTicket&) = delete;

    ~TacHttpTicket() {
        if (ticket != 0 && !settled) {
            TacTickets::cancel(ticket);
        }
    }

private:
    std::string request;
    std::uint32_t ticket = 0;
    bool settled = false;
};

}
