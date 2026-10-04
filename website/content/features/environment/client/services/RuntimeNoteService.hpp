#pragma once
#include <string>

class [[Service]] RuntimeNoteService {
public:
    std::string runtime = "C++, compiled by Emscripten";

    bool showsNotes(bool development) const { return development; }
};
