#[Test(SiteTestSetup, {"partition":"tests"})]
class DrawerServiceTest
  def test_toggle_opens_a_closed_drawer
    drawer = DrawerService.new
    drawer.toggle
    Assert.is_true(drawer.open, "toggle opens a closed drawer")
  end

  def test_only_escape_dismisses
    drawer = DrawerService.new
    drawer.toggle
    drawer.dismiss("Enter")
    Assert.is_true(drawer.open, "only Escape dismisses")
    drawer.dismiss("Escape")
    Assert.equals(false, drawer.open, "Escape dismisses")
  end

  def test_close_closes
    drawer = DrawerService.new
    drawer.toggle
    drawer.close
    Assert.equals(false, drawer.open, "close closes")
  end
end
