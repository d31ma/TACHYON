import Foundation

final class Companion {
  private let system = ProcessInfo.processInfo

  let runtime = "Swift on \(ProcessInfo.processInfo.operatingSystemVersionString)"

  let host = ProcessInfo.processInfo.hostName

  let cores: Int = ProcessInfo.processInfo.processorCount

  func documents() -> String {
    let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
    return paths.first?.path ?? ""
  }

  func lowPower() -> Bool { system.isLowPowerModeEnabled }
}
