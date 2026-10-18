#pragma once
#include "../models/Greeting.hpp"

class [[Test(GreetingTestSetup, {"partition":"tests"})]] GreetingControllerTest {
public:
    void testGetGreetingSaysHello() {
        GreetingController controller = construct<GreetingController>();
        Greeting greeting = controller.getGreeting();
        Assert::equals("Hello from Yon", greeting.message);
    }
};
