#pragma once
#include <string>
#include "../models/Problem.hpp"
#include "../remotes/OrderClient.hpp"

class [[Service]] OrderService {
public:
    int status = 0;
    std::string title;
    int mistakes = 0;
    [[Needs]] OrderClient& orderClient;

    Task<void> send() {
        tachyon::http::HttpRequestException failure("none");
        try {
            co_await orderClient.send(OrderInput{"", 12});
        } catch (const tachyon::http::HttpRequestException& error) {
            failure = error;
        }
        Problem problem = failure.readBodyFromJson<Problem>().value_or(Problem{});
        status = failure.statusCode();
        title = problem.title;
        mistakes = static_cast<int>(problem.errors.size());
    }
};
