#[Test(GreetingTestSetup, {"partition":"tests"})]
class StatusControllerTest
  def test_get_status_says_ok
    raise "expected ok" unless construct(StatusController).get_status == "ok"
  end
end
