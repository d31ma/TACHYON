-- @Test(CatalogTestSetup, {"partition":"tests"})
-- @UseProfiles("test")
---@class CatalogServiceTest
CatalogServiceTest = {}

function CatalogServiceTest:test_uses_a_client_role()
  Assert.equals(8, self:construct(CatalogService):double(4))
end
