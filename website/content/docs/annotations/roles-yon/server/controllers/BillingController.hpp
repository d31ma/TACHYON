#pragma once
#include "../models/Bill.hpp"
#include "../services/BillingService.hpp"

class [[RequestRoute("/billing")]] [[Public]] BillingController {
    [[Needs]] BillingService& service;

public:
    [[GetRoute]]
    Bill getBilling() { return Bill{service.total()}; }
};
