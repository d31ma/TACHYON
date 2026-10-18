#pragma once
#include <string>
#include <vector>
#include "CatalogTestSetup.hpp"

class [[Test(CatalogTestSetup, {"partition":"tests"}), UseProfiles("test"), TestSettings({"tac.catalog.size":5})]] CatalogServiceTest {
public:
    BehaviorContext behaviors;

    void testHasAContext() {
        Assert::equals("tests", behaviors.partition());
        Assert::equals(std::vector<std::string>{"test"}, behaviors.profiles());
    }
};
