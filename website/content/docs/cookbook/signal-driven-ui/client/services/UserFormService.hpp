#pragma once
#include <string>
#include "../remotes/UsersClient.hpp"

class [[Service]] UserFormService {
public:
    std::string name;
    ApplicationEventPublisher applicationEventPublisher;
    [[Needs]] UsersClient& usersClient;

    Task<void> save() {
        UserSaved saved = co_await usersClient.save(name);
        applicationEventPublisher.publishEvent(saved, false);
    }
};
