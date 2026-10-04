struct MacOsTitleBarInsets: Equatable {
    static let none = MacOsTitleBarInsets(height: 0, leading: 0, trailing: 0)

    let height: Int
    let leading: Int
    let trailing: Int

    var pageCall: String {
        "globalThis.__tachyonTitleBar?.apply({\"height\":\(height),\"leading\":\(leading),\"trailing\":\(trailing)})"
    }
}

