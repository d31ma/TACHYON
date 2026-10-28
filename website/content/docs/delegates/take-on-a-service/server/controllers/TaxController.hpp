#pragma once
#include "../models/TaxAnswer.hpp"

class [[RequestRoute("/tax")]] [[Public]] TaxController {
    [[Needs]] TaxDelegate& taxDelegate;

public:
    [[GetRoute]]
    TaxAnswer getTax() {
        TaxAnswer answer;
        answer.rate = taxDelegate.rate("north");
        answer.total = taxDelegate.total(3);
        answer.extra = taxDelegate.total(3, 4);
        return answer;
    }
};
