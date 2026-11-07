struct Summary: Codable {
    let id: Int
    let name: String
    let detail: ShortDetail
    let items: [ShortDetail]
    let kind: String
}
