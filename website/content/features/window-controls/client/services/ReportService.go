// @Service
type ReportService struct {
	// @Needs
	windowControls WindowControls
	Problem        string
}

func (report *ReportService) Minimize() {
	report.keepProblem(report.windowControls.Minimize())
}

func (report *ReportService) Resize() {
	report.keepProblem(report.windowControls.Resize(1280, 800))
}

func (report *ReportService) Rename() {
	report.keepProblem(report.windowControls.SetTitle("Quarterly report"))
}

func (report *ReportService) keepProblem(problem error) {
	if problem != nil {
		report.Problem = problem.Error()
	}
}
