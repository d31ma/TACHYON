#[Test(CatalogTestSetup, {"partition":"tests"})]
#[UseProfiles("test")]
class CatalogServiceTest
  def twice(value)
    value * 2
  end

  def test_doubles_a_value
    raise "expected 8" unless twice(4) == 8
  end
end
