#[Test(GreetingTestSetup, {"partition":"tests"})]
class StatusControllerTest
  def test_get_status_says_ok
    Assert.equals("ok", construct(StatusController).get_status)
  end
end
