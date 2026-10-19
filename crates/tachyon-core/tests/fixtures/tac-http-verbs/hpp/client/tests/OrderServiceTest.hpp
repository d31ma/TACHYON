#pragma once
#include "../services/OrderService.hpp"
#include "../remotes/OrdersClient.hpp"
#include "OrdersTestSetup.hpp"

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
};
