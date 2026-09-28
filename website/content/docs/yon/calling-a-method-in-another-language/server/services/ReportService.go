// @Service
type ReportService struct{}

func (service *ReportService) Summarize(period BehaviorValue) (BehaviorValue, error) {
	return map[string]any{"period": period, "total": 42, "language": "go"}, nil
}
