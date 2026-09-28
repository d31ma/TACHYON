#[Test(SiteTestSetup, {"partition":"tests"})]
class ShowcaseServiceTest
  def test_pick_layer_changes_the_layer
    showcase = ShowcaseService.new
    showcase.start
    raise "Tac first" unless showcase.layer == "tac"

    showcase.pick_layer("yon")
    raise "pick_layer selects a layer" unless showcase.layer == "yon"
  end

  def test_summary_follows_the_layer
    showcase = ShowcaseService.new
    showcase.start
    tac = showcase.summary
    showcase.pick_layer("yon")
    raise "each layer has its own summary" if showcase.summary == tac
  end
end
