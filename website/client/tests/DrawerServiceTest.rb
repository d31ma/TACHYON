#[Test(SiteTestSetup, {"partition":"tests"})]
class DrawerServiceTest
  def test_starts_closed
    drawer = DrawerService.new
    Assert.equals(false, drawer.open, "the drawer starts closed")
  end

  def test_open_state_opens
    drawer = DrawerService.new
    drawer.track("open")
    Assert.is_true(drawer.open, "the open state opens the drawer")
  end

  def test_closed_state_closes
    drawer = DrawerService.new
    drawer.track("open")
    drawer.track("closed")
    Assert.equals(false, drawer.open, "the closed state closes the drawer")
  end

  def test_other_states_close
    drawer = DrawerService.new
    drawer.track("open")
    drawer.track("")
    Assert.equals(false, drawer.open, "a state that is not open closes the drawer")
  end
end
