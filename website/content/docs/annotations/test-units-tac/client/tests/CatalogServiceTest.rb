#[Test(CatalogTestSetup, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest
  def test_uses_a_client_role
    Assert.equals(8, CatalogService.new.double(4))
  end
end
