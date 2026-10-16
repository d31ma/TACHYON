namespace tachyon::http {

class TacTransport {
public:
    static Task<HttpResponseMessage> send(std::string, HttpRequestMessage request) {
        std::string answer = co_await TacHttpTicket(TacBrowserRequest::text(request));
        co_return TacBrowserRequest::response(answer);
    }
};

}
