// @Test(SiteTestSetup, {"partition":"tests"})
type InlineCodeServiceTest struct {
	behaviors *BehaviorContext
}

func (test *InlineCodeServiceTest) TestPartsMarkTheTextBetweenBackticksAsCode() {
	parts := (&InlineCodeService{}).Parts("Run `ty test` before `ty preview`.")
	expected := []InlinePart{
		{Text: "Run ", Code: false},
		{Text: "ty test", Code: true},
		{Text: " before ", Code: false},
		{Text: "ty preview", Code: true},
		{Text: ".", Code: false},
	}
	Assert.Equals(expected, parts)
}

func (test *InlineCodeServiceTest) TestPartsKeepAnUnmatchedBacktickAsText() {
	parts := (&InlineCodeService{}).Parts("A lone ` stays.")
	Assert.Equals([]InlinePart{{Text: "A lone ` stays.", Code: false}}, parts)
}

func (test *InlineCodeServiceTest) TestPartsOfEmptyTextAreEmpty() {
	Assert.Equals(0, len((&InlineCodeService{}).Parts("")), "empty text has no parts")
}
