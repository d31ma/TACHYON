#[Test(CatalogTestSetup, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest
  def twice(value)
    value * 2
  end

  def test_doubles_a_value
    Assert.equals(8, twice(4))
  end
end
