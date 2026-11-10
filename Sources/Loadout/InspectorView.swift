import AppKit
import LoadoutCore
import SwiftUI

struct InspectorView: View {
    let items: [Item]
    let requestRemoval: ([Item]) -> Void

    var body: some View {
        if items.count == 1, let item = items.first {
            detail(item)
        } else if items.count > 1 {
            VStack(spacing: 12) {
                Text("\(items.count) items selected").font(.headline)
                Text(Format.bytes(items.reduce(0) { $0 + $1.bytes }) + " on disk").foregroundStyle(.secondary)
                Button("Remove \(items.count) Items…", role: .destructive) {
                    requestRemoval(items.filter { $0.removal.isSupported })
                }
                .disabled(!items.contains { $0.removal.isSupported })
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            ContentUnavailableView("No Selection", systemImage: "sidebar.right",
                                   description: Text("Select an item to see where it lives and what it costs."))
        }
    }

    private func detail(_ item: Item) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Label(item.name, systemImage: item.kind.symbol)
                    .font(.title3.weight(.semibold))
                    .textSelection(.enabled)
                Text("\(item.agent) · \(item.kind.title)").foregroundStyle(.secondary)

                if !item.summary.isEmpty {
                    Text(item.summary).textSelection(.enabled)
                }

                Grid(alignment: .leadingFirstTextBaseline, horizontalSpacing: 10, verticalSpacing: 6) {
                    row("Size", item.bytes > 0 ? Format.bytes(item.bytes) : "—")
                    row("Tokens", item.tokens > 0 ? "~\(item.tokens)" + (item.perSession ? "" : " (project only)") : "—")
                    if let path = item.path { row("Path", path.path) }
                    if let note = item.note { row("Note", note) }
                }
                .font(.callout)

                HStack {
                    if let path = item.path {
                        Button("Reveal in Finder") { NSWorkspace.shared.activateFileViewerSelecting([path]) }
                        Button("Open") { NSWorkspace.shared.open(path) }
                    }
                }

                VStack(alignment: .leading, spacing: 6) {
                    Button("Remove…", role: .destructive) { requestRemoval([item]) }
                        .disabled(!item.removal.isSupported)
                    Text(item.removal.explanation).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }

                if let preview = Preview.text(for: item) {
                    Divider()
                    Text(preview)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding()
        }
    }

    private func row(_ label: String, _ value: String) -> some View {
        GridRow {
            Text(label).foregroundStyle(.secondary)
            Text(value).textSelection(.enabled)
        }
    }
}

enum Preview {
    /// First part of the file behind an item: SKILL.md for skill folders, the file itself otherwise.
    static func text(for item: Item) -> String? {
        guard let path = item.path else { return nil }
        var file = path
        var isDir: ObjCBool = false
        if FileManager.default.fileExists(atPath: path.path, isDirectory: &isDir), isDir.boolValue {
            file = path.appendingPathComponent(item.kind == .memory ? "MEMORY.md" : "SKILL.md")
        }
        guard ["md", "toml", "mdc", "txt"].contains(file.pathExtension),
              let handle = try? FileHandle(forReadingFrom: file) else { return nil }
        defer { try? handle.close() }
        let data = (try? handle.read(upToCount: 6000)) ?? Data()
        return String(decoding: data, as: UTF8.self)
    }
}
