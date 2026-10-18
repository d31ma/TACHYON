#pragma once
#include <string>
#include <vector>

class [[Test(CatalogTestSetup, {"partition":"tests"})]] [[UseProfiles("test")]] CatalogServiceTest {
public:
    BehaviorContext behaviors;
    [[Fake]] CatalogCounterHelper catalogCounterHelper;

    void testDoublesTheCount() {
        catalogCounterHelper.value = 4;
        CatalogService service = construct<CatalogService>();
        Assert::equals(8, service.total());
    }

    void testControllerKeepsTheRealHelper() {
        catalogCounterHelper.value = 4;
        CatalogController controller = construct<CatalogController>();
        Assert::equals(42, controller.getTotal().total);
    }

    void testHasAContext() {
        Assert::equals("tests", behaviors.partition());
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
    }
};
