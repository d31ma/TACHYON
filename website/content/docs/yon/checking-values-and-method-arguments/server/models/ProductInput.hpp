#pragma once
#include <optional>
#include <string>

struct [[Checked]] ProductInput {
    [[NotEmpty, Length({"min":1,"max":80})]] std::string name;
    [[Required, Min(1), Max(9)]] std::optional<int> level;
};
