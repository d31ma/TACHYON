import "strconv"

// @Service
type TallyService struct {
	Count int64
}

func (tally *TallyService) Add(amount int64) {
	tally.Count += amount
}

func (tally *TallyService) Summary(unit string) string {
	return strconv.FormatInt(tally.Count, 10) + " " + unit
}
