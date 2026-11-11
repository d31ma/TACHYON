#pragma once
#include <stdexcept>
#include <string>

class [[Service]] RenameService {
public:
    bool editing = false;
    std::string problem;
    [[Needs]] Focus& focus;

    void open() {
        editing = true;
        try {
            focus.trap("dialog");
            focus.move("name");
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    void close() {
        editing = false;
        try {
            focus.release();
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    [[OnEvent]]
    void failed(const BrowserCallFailed& failure) { problem = failure.message; }
};
