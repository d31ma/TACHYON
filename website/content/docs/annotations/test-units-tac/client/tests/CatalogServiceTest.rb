#[Test(CatalogTestSetup, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest
  def test_uses_a_client_role
    raise "expected 8" unless CatalogService.new.double(4) == 8
  end
end
