enum NativeUiStylePhase {
    case idle
    case invalidating(NativeUiStyleTicket?)
    case preparing(NativeUiStyleTicket)
    case prepared(NativeUiStyleTicket)
    case committing(NativeUiStyleTicket)
    case committed(NativeUiStyleTicket)
    case rollingBack(NativeUiStyleTicket)
    case uncertain(NativeUiStyleTicket)

    func ticket() -> NativeUiStyleTicket? { extract() }
    func mayPrepare() -> Bool { preparable() }
    func mayCommit() -> Bool { committable() }
    func mayRollback() -> Bool { rollbackable() }
    func mayReconcile() -> Bool { reconcilable() }

    private func extract() -> NativeUiStyleTicket? {
        switch self {
        case .idle: return nil
        case .invalidating(let value): return value
        case .preparing(let value), .prepared(let value), .committing(let value), .committed(let value), .rollingBack(let value), .uncertain(let value): return value
        }
    }

    private func preparable() -> Bool {
        switch self { case .idle, .committed: return true; default: return false }
    }

    private func committable() -> Bool {
        switch self { case .prepared, .committed: return true; default: return false }
    }
    private func rollbackable() -> Bool {
        switch self { case .prepared, .committed, .uncertain: return true; default: return false }
    }

    private func reconcilable() -> Bool {
        switch self { case .uncertain: return true; default: return false }
    }
}
