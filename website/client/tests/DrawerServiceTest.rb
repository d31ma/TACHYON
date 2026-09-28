#[Test(SiteTestSetup, {"partition":"tests"})]
class DrawerServiceTest
  def test_toggle_close_and_dismiss_drive_open
    drawer = DrawerService.new
    drawer.toggle
    raise "toggle opens a closed drawer" unless drawer.open

    drawer.dismiss("Enter")
    raise "only Escape dismisses" unless drawer.open

    drawer.dismiss("Escape")
    raise "Escape dismisses" if drawer.open

    drawer.toggle
    drawer.close
    raise "close closes" if drawer.open
  end
end
