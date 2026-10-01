import Foundation
import Darwin

struct NativeUiRuntimeFile {
    static func read(_ directory: Int32, _ name: String) throws -> Data {
        let descriptor = try opened(directory, name)
        let file = FileHandle(fileDescriptor: descriptor, closeOnDealloc: true)
        try metadata(descriptor)
        return try bounded(file)
    }

    private static func opened(_ parent: Int32, _ name: String) throws -> Int32 {
        let descriptor = openat(parent, name, O_RDONLY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK)
        try NativeUiRuntimeFailure.require(descriptor >= 0, "Cannot open contained native UI asset")
        return descriptor
    }

    private static func metadata(_ descriptor: Int32) throws {
        var metadata = stat()
        let result = fstat(descriptor, &metadata)
        try NativeUiRuntimeFailure.require(result == 0, "Cannot inspect opened asset")
        try NativeUiRuntimeFailure.require(metadata.st_mode & mode_t(S_IFMT) == mode_t(S_IFREG), "Captured asset must be a regular file")
        try NativeUiRuntimeFailure.require(metadata.st_size >= 0 && metadata.st_size <= off_t(NativeUiRuntimeBounds.assetBytes), "Asset byte limit exceeded")
    }

    private static func bounded(_ file: FileHandle) throws -> Data {
        var result = Data()
        var remaining = NativeUiRuntimeBounds.assetBytes + 1
        while let chunk = try file.read(upToCount: min(remaining, 65536)), !chunk.isEmpty {
            result.append(chunk)
            remaining -= chunk.count
            try NativeUiRuntimeFailure.require(remaining > 0, "Asset byte limit exceeded")
        }
        return result
    }
}
