#pragma once

class [[Test(StatusTestSetup, {"partition":"tests"})]] StatusControllerTest {
public:
    BehaviorContext behaviors;

    void testGetStatusSaysOk() {
        StatusController controller = construct<StatusController>();
        Assert::equals("ok", controller.getStatus());
    }
};
