import Foundation
import Darwin

final class NativeUiRuntimeDirectory {
    let descriptor: Int32

    init(borrowing descriptor: Int32) throws {
        self.descriptor = try Self.opened(descriptor, ".")
    }

    private init(owned descriptor: Int32) { self.descriptor = descriptor }

    deinit { Darwin.close(descriptor) }

    func child(_ name: String) throws -> NativeUiRuntimeDirectory {
        let opened = try Self.opened(descriptor, name)
        return NativeUiRuntimeDirectory(owned: opened)
    }

    func names() throws -> [String] {
        let independent = try Self.opened(descriptor, ".")
        let stream = try Self.stream(independent)
        defer { closedir(stream) }
        return try Self.entries(stream)
    }

    func isDirectory(_ name: String) throws -> Bool {
        var metadata = stat()
        let result = fstatat(descriptor, name, &metadata, AT_SYMLINK_NOFOLLOW)
        try NativeUiRuntimeFailure.require(result == 0, "Cannot inspect captured entry")
        try Self.regular(metadata.st_mode)
        return metadata.st_mode & mode_t(S_IFMT) == mode_t(S_IFDIR)
    }

    private static func opened(_ parent: Int32, _ name: String) throws -> Int32 {
        let opened = openat(parent, name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK)
        try NativeUiRuntimeFailure.require(opened >= 0, "Cannot open contained native UI directory")
        return opened
    }

    private static func stream(_ descriptor: Int32) throws -> UnsafeMutablePointer<DIR> {
        guard let stream = fdopendir(descriptor) else {
            Darwin.close(descriptor)
            throw NativeUiRuntimeFailure(description: "Cannot enumerate captured directory")
        }
        return stream
    }

    private static func entries(_ stream: UnsafeMutablePointer<DIR>) throws -> [String] {
        var names: [String] = []
        errno = 0
        while let entry = readdir(stream) {
            try append(entry, &names)
            errno = 0
        }
        try NativeUiRuntimeFailure.require(errno == 0, "Cannot complete captured directory enumeration")
        return names
    }

    private static func append(_ entry: UnsafeMutablePointer<dirent>, _ names: inout [String]) throws {
        let name = try entryName(entry)
        names.append(name)
        try NativeUiRuntimeFailure.require(names.count <= NativeUiRuntimeBounds.entries + 2, "Directory entry limit exceeded")
    }

    private static func entryName(_ entry: UnsafeMutablePointer<dirent>) throws -> String {
        let name = withUnsafePointer(to: &entry.pointee.d_name) { pointer in
            pointer.withMemoryRebound(to: CChar.self, capacity: Int(entry.pointee.d_namlen) + 1) { String(validatingCString: $0) }
        }
        guard let name else { throw NativeUiRuntimeFailure(description: "Asset name must be UTF-8") }
        return name
    }

    private static func regular(_ mode: mode_t) throws {
        let kind = mode & mode_t(S_IFMT)
        try NativeUiRuntimeFailure.require([mode_t(S_IFDIR), mode_t(S_IFREG)].contains(kind), "Symlinks and special assets are forbidden")
    }
}
