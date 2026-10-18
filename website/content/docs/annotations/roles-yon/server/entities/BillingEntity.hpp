#pragma once

struct [[Entity({"adapter":"billing"}), Table("invoices")]] BillingEntity {
    [[Id]] long long id = 0;
};
