#pragma once

struct [[Entity({"adapter":"indexeddb"}), Table("invoices")]] BillingEntity {
    [[Id]] long long id = 0;
};
