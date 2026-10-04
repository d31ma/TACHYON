struct Product: Codable {
    let id: Int
    let name: String
    let detail: Detail
    let items: [Detail]
    let secret: String
}
