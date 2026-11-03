#pragma once
#include <string>
#include <vector>

class [[Test(CatalogTestSetup, {"partition":"tests"})]] [[UseProfiles("test")]] CatalogServiceTest {
public:
    BehaviorContext behaviors;
    [[Fake]] CatalogCounterDelegate catalogCounterDelegate;

    void testDoublesTheCount() {
        catalogCounterDelegate.value = 4;
        CatalogService service = construct<CatalogService>();
        Assert::equals(8, service.total());
    }

    void testControllerKeepsTheRealDelegate() {
        catalogCounterDelegate.value = 4;
        CatalogController controller = construct<CatalogController>();
        Assert::equals(42, controller.getTotal().total);
    }

    void testHasAContext() {
        Assert::equals("tests", behaviors.partition());
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
    }
};
