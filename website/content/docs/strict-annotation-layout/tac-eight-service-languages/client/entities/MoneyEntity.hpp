#pragma once

struct [[Entity({"adapter":"publicMoney"}), Table("money")]] MoneyEntity {
    [[Id]] long long id = 0;
};
