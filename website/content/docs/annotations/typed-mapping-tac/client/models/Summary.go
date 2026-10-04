type Summary struct {
	ID     int           `json:"id"`
	Name   string        `json:"name"`
	Detail ShortDetail   `json:"detail"`
	Items  []ShortDetail `json:"items"`
	Kind   string        `json:"kind"`
}
