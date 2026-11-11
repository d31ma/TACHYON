#pragma once
#include "../services/OrderService.hpp"
#include "../remotes/OrdersClient.hpp"
#include "OrdersTestSetup.hpp"
#include "../models/ClaimConflict.hpp"

class [[Test(OrdersTestSetup, {"partition":"tac-http-verbs"})]] OrderServiceTest {
public:
    [[Fake]] OrdersClient ordersClient;

    Task<void> testPutSendsThePathTheHeaderAndTheBody() {
        OrderService service = construct<OrderService>();
        SentRequest sent = co_await service.replace();
        Assert::equals("PUT", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("north", sent.header);
        Assert::equals("hammer", sent.item);
    }

    Task<void> testPatchSendsThePathTheHeaderAndTheBody() {
        OrderService service = construct<OrderService>();
        SentRequest sent = co_await service.change();
        Assert::equals("PATCH", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("south", sent.header);
        Assert::equals("nails", sent.item);
    }

    Task<void> testDeleteSendsThePathAndTheHeaderWithoutABody() {
        OrderService service = construct<OrderService>();
        SentRequest sent = co_await service.remove();
        Assert::equals("DELETE", sent.method);
        Assert::equals("/api/orders/a-7", sent.path);
        Assert::equals("duplicate", sent.header);
        Assert::equals("none", sent.item);
    }

    Task<void> testErrorBodyIsReadIntoAModel() {
        OrderService service = construct<OrderService>();
        tachyon::http::HttpRequestException failure("none");
        try {
            co_await service.claim();
        } catch (const tachyon::http::HttpRequestException& error) {
            failure = error;
        }
        ClaimConflict conflict = failure.readBodyFromJson<ClaimConflict>().value_or(ClaimConflict{});
        Assert::equals("HTTP_STATUS", failure.code());
        Assert::equals(409, failure.statusCode());
        Assert::equals("The HTTP request failed.", std::string(failure.what()));
        Assert::equals("taken", conflict.reason);
        Assert::equals("south", conflict.holder);
    }
};
