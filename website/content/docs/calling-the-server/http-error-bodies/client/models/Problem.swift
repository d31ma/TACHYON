struct Problem: Codable {
    let title: String
    let status: Int
    let errors: [ProblemEntry]
}
