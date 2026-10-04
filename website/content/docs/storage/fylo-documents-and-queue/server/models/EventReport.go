type EventReport struct {
	Found   string `json:"found"`
	Patched int    `json:"patched"`
	Pages   int    `json:"pages"`
	Seen    int    `json:"seen"`
	Job     Job    `json:"job"`
	Gone    bool   `json:"gone"`
	Dropped bool   `json:"dropped"`
}
