import Foundation
import WebKit

@MainActor final class NativeUiStyleSession {
    private let view: WKWebView
    private let assets: NativeUiStyleAssets
    private var epoch: UInt64 = 0
    private var phase = NativeUiStylePhase.idle

    init(_ view: WKWebView, _ assets: NativeUiStyleAssets) { self.view = view; self.assets = assets }

    func prepare(_ candidate: NativeUiRuntimeGeneration) async throws -> NativeUiStyleTicket {
        try assets.validate(candidate)
        try NativeUiRuntimeFailure.require(phase.mayPrepare(), "Another native stylesheet operation is active")
        let previous = phase.ticket()
        let ticket = try nextTicket(candidate)
        let arguments = try assets.stylesheetArguments(candidate)
        _ = try NativeUiStyleCall.checked("prepare", ticket, arguments)
        phase = .preparing(ticket)
        try await finalize(previous, ticket)
        return try await staged(candidate, ticket, arguments)
    }

    func commit(_ ticket: NativeUiStyleTicket) async throws {
        try current(ticket)
        try NativeUiRuntimeFailure.require(phase.mayCommit(), "Stylesheets cannot commit in this native phase")
        phase = .committing(ticket)
        try await committed(ticket)
    }

    func rollback(_ ticket: NativeUiStyleTicket) async throws {
        try current(ticket)
        try NativeUiRuntimeFailure.require(phase.mayRollback(), "Stylesheet operation is still in flight")
        phase = .rollingBack(ticket)
        try await rolledBack(ticket)
    }

    func reconcile(_ ticket: NativeUiStyleTicket) async throws {
        try current(ticket)
        try NativeUiRuntimeFailure.require(phase.mayReconcile(), "Stylesheet operation is not uncertain")
        let result = try await acknowledged("status", ticket)
        try reconciled(result, ticket)
    }

    func invalidate() async {
        epoch += 1
        let invalidationEpoch = epoch
        let previous = phase.ticket()
        phase = .invalidating(previous)
        await invalidated(previous)
        completeInvalidation(invalidationEpoch)
    }

    private func nextTicket(_ candidate: NativeUiRuntimeGeneration) throws -> NativeUiStyleTicket {
        guard let url = view.url else { throw NativeUiRuntimeFailure(description: "No owned main-frame document") }
        _ = try NativeUiStyleAddress.path(url)
        return NativeUiStyleTicket(token: UUID().uuidString, epoch: epoch, url: url.absoluteString, generation: candidate.descriptor.generation)
    }

    private func finalize(_ previous: NativeUiStyleTicket?, _ next: NativeUiStyleTicket) async throws {
        do {
            try await NativeUiStyleCall.finalize(view, previous)
            try current(next)
        } catch {
            finalizationFailed(previous, next)
            throw error
        }
    }

    private func staged(_ candidate: NativeUiRuntimeGeneration, _ ticket: NativeUiStyleTicket, _ arguments: [String: [String: String]]) async throws -> NativeUiStyleTicket {
        try stage(candidate, ticket)
        do {
            let result = try await NativeUiStyleCall.run(view, "prepare", ticket, arguments)
            try current(ticket)
            try NativeUiRuntimeFailure.require(result == "prepared", "Stylesheet preparation was not acknowledged")
            phase = .prepared(ticket)
            return ticket
        } catch {
            uncertain(ticket)
            throw error
        }
    }

    private func stage(_ candidate: NativeUiRuntimeGeneration, _ ticket: NativeUiStyleTicket) throws {
        do { try assets.stage(candidate) } catch {
            abandonStage(ticket)
            throw error
        }
    }

    private func abandonStage(_ ticket: NativeUiStyleTicket) {
        if phase.ticket() == ticket && epoch == ticket.epoch {
            assets.discard()
            phase = .idle
        }
    }

    private func completeInvalidation(_ invalidationEpoch: UInt64) {
        if epoch == invalidationEpoch {
            assets.discard()
            phase = .idle
        }
    }

    private func acknowledged(_ operation: String, _ ticket: NativeUiStyleTicket) async throws -> String {
        do {
            let result = try await NativeUiStyleCall.run(view, operation, ticket)
            try current(ticket)
            return result
        } catch {
            uncertain(ticket)
            throw error
        }
    }

    private func committed(_ ticket: NativeUiStyleTicket) async throws {
        do {
            let result = try await acknowledged("commit", ticket)
            try NativeUiRuntimeFailure.require(result == "committed", "Stylesheet commit was not acknowledged")
            try assets.promote(ticket.generation)
            phase = .committed(ticket)
        } catch {
            uncertain(ticket)
            throw error
        }
    }

    private func rolledBack(_ ticket: NativeUiStyleTicket) async throws {
        do {
            try assets.prepareRollback(ticket.generation)
            let result = try await acknowledged("rollback", ticket)
            try NativeUiRuntimeFailure.require(result == "rolledback", "Stylesheet rollback was not acknowledged")
            try assets.rollback(ticket.generation)
            phase = .idle
        } catch {
            uncertain(ticket)
            throw error
        }
    }

    private func current(_ ticket: NativeUiStyleTicket) throws {
        try NativeUiRuntimeFailure.require(phase.ticket() == ticket && epoch == ticket.epoch && view.url?.absoluteString == ticket.url, "Stale native stylesheet ticket")
    }

    private func finalizationFailed(_ previous: NativeUiStyleTicket?, _ next: NativeUiStyleTicket) {
        if phase.ticket() == next && epoch == next.epoch {
            phase = previous.map { .uncertain($0) } ?? .idle
        }
    }

    private func uncertain(_ ticket: NativeUiStyleTicket) {
        if phase.ticket() == ticket && epoch == ticket.epoch { phase = .uncertain(ticket) }
    }

    private func reconciled(_ result: String, _ ticket: NativeUiStyleTicket) throws {
        switch result {
        case "prepared": phase = .prepared(ticket)
        case "committed": try assets.promote(ticket.generation); phase = .committed(ticket)
        case "rolledback": try assets.rollback(ticket.generation); phase = .idle
        case "finalized": assets.discard(); phase = .idle
        default: throw NativeUiRuntimeFailure(description: "Stylesheet operation is still pending or unavailable")
        }
    }

    private func invalidated(_ previous: NativeUiStyleTicket?) async {
        guard let previous else { return }
        _ = try? await NativeUiStyleCall.run(view, "invalidate", previous)
    }
}
