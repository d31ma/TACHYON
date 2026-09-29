#[Test(SiteTestSetup, {"partition":"tests"})]
class ShowcaseServiceTest
  def test_pick_layer_changes_the_layer
    showcase = ShowcaseService.new
    showcase.start
    Assert.equals("tac", showcase.layer, "Tac first")
    showcase.pick_layer("yon")
    Assert.equals("yon", showcase.layer, "pick_layer selects a layer")
  end

  def test_summary_follows_the_layer
    showcase = ShowcaseService.new
    showcase.start
    tac = showcase.summary
    showcase.pick_layer("yon")
    Assert.is_true(showcase.summary != tac, "each layer has its own summary")
  end
end
