#pragma once
#include "../models/ProgramState.hpp"

class [[HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})]] PageProgramClient {
public:
    [[GetRequest("/read")]]
    Task<ProgramState> read();

    [[PostRequest("/increment")]]
    Task<ProgramState> increment();
};
