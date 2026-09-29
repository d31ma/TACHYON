final class SecureRandom {
    func bytes(_ count: Int) throws -> [UInt8] {
        let values = try TacWebHost.answer(TacWebHost.call("secureRandom", "bytes", [count]), as: [Any].self)
        return try values.map { UInt8(truncatingIfNeeded: try TacWebHost.answer($0, as: Int.self)) }
    }

    func uuid() throws -> String {
        try TacWebHost.text("secureRandom", "uuid")
    }
}
