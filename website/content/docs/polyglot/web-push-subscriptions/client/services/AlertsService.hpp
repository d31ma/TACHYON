#pragma once
#include <stdexcept>
#include <string>

class [[Service]] AlertsService {
public:
    std::string endpoint;
    std::string problem;
    [[Needs]] Push& push;

    void turnOn() {
        try {
            push.subscribe(serverKey);
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    void turnOff() {
        try {
            push.unsubscribe();
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    [[OnEvent]]
    void subscribed(const PushSubscribed& subscribed) { endpoint = subscribed.endpoint; }

    [[OnEvent]]
    void unsubscribed(const PushUnsubscribed&) { endpoint = ""; }

    [[OnEvent]]
    void failed(const BrowserCallFailed& failure) { problem = failure.message; }

private:
    static constexpr const char* serverKey = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U";
};
