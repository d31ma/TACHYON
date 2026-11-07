#pragma once
#include <string>
#include "../entities/ProgramNoteEntity.hpp"

class [[Store(ProgramNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})]] ProgramNoteStore : public tachyon::Repository<ProgramNoteEntity, std::string> {
};
