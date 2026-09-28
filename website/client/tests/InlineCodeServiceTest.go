import (
	"errors"
	"fmt"
	"slices"
)

// @Test(SiteTestSetup, {"partition":"tests"})
type InlineCodeServiceTest struct {
	behaviors *BehaviorContext
}

func (test *InlineCodeServiceTest) TestPartsMarkTheTextBetweenBackticksAsCode() error {
	parts := (&InlineCodeService{}).Parts("Run `ty test` before `ty preview`.")
	expected := []InlinePart{
		{Text: "Run ", Code: false},
		{Text: "ty test", Code: true},
		{Text: " before ", Code: false},
		{Text: "ty preview", Code: true},
		{Text: ".", Code: false},
	}
	if !slices.Equal(parts, expected) {
		return fmt.Errorf("%v", parts)
	}
	return nil
}

func (test *InlineCodeServiceTest) TestPartsKeepAnUnmatchedBacktickAsText() error {
	parts := (&InlineCodeService{}).Parts("A lone ` stays.")
	if !slices.Equal(parts, []InlinePart{{Text: "A lone ` stays.", Code: false}}) {
		return fmt.Errorf("%v", parts)
	}
	return nil
}

func (test *InlineCodeServiceTest) TestPartsOfEmptyTextAreEmpty() error {
	if len((&InlineCodeService{}).Parts("")) != 0 {
		return errors.New("empty text has no parts")
	}
	return nil
}
