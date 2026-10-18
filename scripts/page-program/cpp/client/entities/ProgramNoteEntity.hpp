#pragma once
#include <string>

struct [[Entity({"adapter":"indexeddb"}), Table("programNotes")]] ProgramNoteEntity {
    [[Id]] std::string id;
    std::string text;
};
