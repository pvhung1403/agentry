import Foundation

public enum ActionError: LocalizedError {
    case toolNotFound(String)
    case commandFailed(String)
    case notJSONObject(String)
    case unsupported(String)

    public var errorDescription: String? {
        switch self {
        case .toolNotFound(let tool): "Could not find the `\(tool)` command. Install it or add it to your PATH."
        case .commandFailed(let output): output.isEmpty ? "The command failed." : output
        case .notJSONObject(let path): "\(path) is not a JSON object, so it was not changed."
        case .unsupported(let reason): reason
        }
    }
}

/// Changes things on disk. Every change is reversible: files go to the Trash, config edits are backed up.
public struct Actions: Sendable {
    public let home: URL
    public let backupRoot: URL

    public init(home: URL = FileManager.default.homeDirectoryForCurrentUser, backupRoot: URL? = nil) {
        self.home = home
        self.backupRoot = backupRoot
            ?? home.appendingPathComponent("Library/Application Support/Loadout/Backups")
    }

    public func remove(_ item: Item) async throws {
        switch item.removal {
        case .trash(let url): try FileManager.default.trashItem(at: url, resultingItemURL: nil)
        case .cli(let argv, let cwd): try await run(argv, cwd: cwd)
        case .jsonKey(let file, let path): try removeJSONKey(file: file, path: path)
        case .unsupported(let reason): throw ActionError.unsupported(reason)
        }
    }

    public func setEnabled(_ item: Item, _ enabled: Bool) async throws {
        guard let toggle = item.toggle else { return }
        try await run(enabled ? toggle.enable : toggle.disable, cwd: nil)
    }

    // MARK: - JSON config edits

    func removeJSONKey(file: URL, path: [String]) throws {
        let data = try Data(contentsOf: file)
        guard let root = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw ActionError.notJSONObject(file.path)
        }
        try backup(file)
        let updated = Self.removing(path[...], from: root)
        // Note: key order and formatting are not preserved; the backup keeps the original.
        let output = try JSONSerialization.data(withJSONObject: updated,
                                                options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes])
        try output.write(to: file, options: .atomic)
    }

    static func removing(_ path: ArraySlice<String>, from dict: [String: Any]) -> [String: Any] {
        guard let key = path.first else { return dict }
        var copy = dict
        if path.count == 1 {
            copy.removeValue(forKey: key)
        } else if let child = copy[key] as? [String: Any] {
            copy[key] = removing(path.dropFirst(), from: child)
        }
        return copy
    }

    @discardableResult
    func backup(_ file: URL) throws -> URL {
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        let folder = backupRoot.appendingPathComponent("\(stamp)-\(UUID().uuidString.prefix(6))")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let relative = file.path.hasPrefix(home.path + "/") ? String(file.path.dropFirst(home.path.count + 1)) : file.lastPathComponent
        let copy = folder.appendingPathComponent(relative.replacingOccurrences(of: "/", with: "__"))
        try FileManager.default.copyItem(at: file, to: copy)
        return copy
    }

    // MARK: - Running agent CLIs

    /// Apps opened from Finder don't get the shell's PATH, so also look in the usual install folders.
    var searchPath: [String] {
        let fromHome = [".local/bin", ".bun/bin", ".npm-global/bin", ".volta/bin", ".cargo/bin"].map { home.appendingPathComponent($0).path }
        let system = ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
        let inherited = (ProcessInfo.processInfo.environment["PATH"] ?? "").split(separator: ":").map(String.init)
        return fromHome + system + inherited
    }

    func resolve(_ tool: String) -> URL? {
        searchPath.lazy
            .map { URL(fileURLWithPath: $0).appendingPathComponent(tool) }
            .first { FileManager.default.isExecutableFile(atPath: $0.path) }
    }

    func run(_ argv: [String], cwd: URL?) async throws {
        guard let tool = argv.first else { return }
        guard let executable = resolve(tool) else { throw ActionError.toolNotFound(tool) }
        var environment = ProcessInfo.processInfo.environment
        environment["PATH"] = searchPath.joined(separator: ":")
        let arguments = Array(argv.dropFirst())
        let env = environment

        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            DispatchQueue.global(qos: .userInitiated).async {
                let process = Process()
                process.executableURL = executable
                process.arguments = arguments
                process.environment = env
                if let cwd { process.currentDirectoryURL = cwd }
                let pipe = Pipe()
                process.standardOutput = pipe
                process.standardError = pipe
                process.standardInput = FileHandle.nullDevice
                do {
                    try process.run()
                    // Read before waiting so a chatty command can't fill the pipe and hang.
                    let output = pipe.fileHandleForReading.readDataToEndOfFile()
                    process.waitUntilExit()
                    if process.terminationStatus == 0 {
                        continuation.resume()
                    } else {
                        let text = String(decoding: output, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
                        continuation.resume(throwing: ActionError.commandFailed(text))
                    }
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
}
