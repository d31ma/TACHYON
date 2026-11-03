#pragma once
#include <string>
#include "../models/ProgramState.hpp"
#include "../remotes/PageProgramClient.hpp"
#include "../stores/ProgramNoteStore.hpp"

class [[Service]] WriterService {
public:
    std::string identity;
    int count = 0;
    std::string recalled;
    [[Needs]] PageProgramClient& pageProgramClient;
    [[Needs]] ProgramNoteStore& programNoteStore;

    Task<void> inspect() {
        show(co_await pageProgramClient.read());
    }

    Task<void> increment() {
        show(co_await pageProgramClient.increment());
    }

    void remember(std::string text) {
        programNoteStore.save(ProgramNoteEntity{NOTE, text});
    }

    void recall() {
        auto note = programNoteStore.findById(NOTE);
        recalled = note ? note->text : std::string();
    }

private:
    static constexpr const char* NOTE = "note";

    void show(const ProgramState& state) {
        identity = state.identity;
        count = state.count;
    }
};
