type Product struct {
	ID     int      `json:"id"`
	Name   string   `json:"name"`
	Detail Detail   `json:"detail"`
	Items  []Detail `json:"items"`
	Secret string   `json:"secret"`
}
