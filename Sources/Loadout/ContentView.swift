import AppKit
import LoadoutCore
import SwiftUI

struct ContentView: View {
    @Environment(Store.self) private var store
    @State private var sidebar: SidebarSelection? = .all
    @State private var selection = Set<Item.ID>()
    @State private var search = ""
    @State private var sortOrder = [KeyPathComparator(\Item.bytes, order: .reverse)]
    @State private var showInspector = true
    @State private var pendingRemoval: [Item] = []

    private var visible: [Item] {
        let base = store.items(for: sidebar ?? .all)
        let matches = search.isEmpty ? base : base.filter {
            $0.name.localizedCaseInsensitiveContains(search) || $0.summary.localizedCaseInsensitiveContains(search)
        }
        return matches.sorted(using: sortOrder)
    }

    private var selectedItems: [Item] { store.items.filter { selection.contains($0.id) } }

    var body: some View {
        NavigationSplitView {
            sidebarList
                .navigationSplitViewColumnWidth(min: 200, ideal: 220)
        } detail: {
            VStack(spacing: 0) {
                SummaryBar(items: visible) { pendingRemoval = store.items(for: .kind(.leftover)) }
                Divider()
                itemTable
            }
            .inspector(isPresented: $showInspector) {
                InspectorView(items: selectedItems) { pendingRemoval = $0 }
                    .inspectorColumnWidth(min: 280, ideal: 340, max: 520)
            }
        }
        .searchable(text: $search, prompt: "Search name or description")
        .toolbar { toolbar }
        .confirmationDialog(removalTitle, isPresented: removalBinding, titleVisibility: .visible) {
            Button("Remove", role: .destructive) {
                let targets = pendingRemoval
                selection.subtract(targets.map(\.id))
                Task { await store.remove(targets) }
            }
        } message: {
            Text(pendingRemoval.prefix(8).map { "• \($0.name): \($0.removal.explanation)" }.joined(separator: "\n")
                 + (pendingRemoval.count > 8 ? "\n…and \(pendingRemoval.count - 8) more." : ""))
        }
        .alert("Some actions failed", isPresented: errorBinding) {
            Button("OK") { store.errorMessage = nil }
        } message: {
            Text(store.errorMessage ?? "")
        }
        .task { await store.refresh() }
    }

    // MARK: - Sidebar

    private var sidebarList: some View {
        List(selection: $sidebar) {
            Section("Library") {
                sidebarRow(.all, title: "All Items", symbol: "square.grid.2x2", count: store.items.count)
                ForEach(Kind.allCases, id: \.self) { kind in
                    let count = store.items(for: .kind(kind)).count
                    if count > 0 { sidebarRow(.kind(kind), title: kind.title, symbol: kind.symbol, count: count) }
                }
            }
            Section("Agents") {
                ForEach(store.agents, id: \.self) { agent in
                    sidebarRow(.agent(agent), title: agent, symbol: "cpu", count: store.items(for: .agent(agent)).count)
                }
            }
        }
    }

    private func sidebarRow(_ value: SidebarSelection, title: String, symbol: String, count: Int) -> some View {
        Label(title, systemImage: symbol)
            .badge(count)
            .tag(value)
    }

    // MARK: - Table

    private var itemTable: some View {
        Table(visible, selection: $selection, sortOrder: $sortOrder) {
            TableColumn("Name", value: \.name) { item in
                Label {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(item.name).lineLimit(1)
                        if !item.summary.isEmpty {
                            Text(item.summary).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                    }
                } icon: {
                    Image(systemName: item.kind.symbol).foregroundStyle(item.kind == .leftover ? .orange : .accentColor)
                }
            }
            .width(min: 220, ideal: 340)
            TableColumn("Agent", value: \.agent).width(min: 90, ideal: 120)
            TableColumn("Kind", value: \.kind.rawValue) { Text($0.kind.rawValue.capitalized) }.width(min: 70, ideal: 90)
            TableColumn("Size", value: \.bytes) { item in
                Text(item.bytes > 0 ? Format.bytes(item.bytes) : "—").monospacedDigit()
            }
            .width(min: 60, ideal: 80)
            TableColumn("Tokens", value: \.tokens) { item in
                Text(item.tokens > 0 ? Format.tokens(item.tokens) : "—").monospacedDigit()
                    .foregroundStyle(item.sessionTokens > 0 ? .primary : .secondary)
            }
            .width(min: 60, ideal: 80)
            TableColumn("Status") { item in
                StatusCell(item: item) { enabled in Task { await store.setEnabled(item, enabled) } }
            }
                .width(min: 80, ideal: 110)
        }
        .contextMenu(forSelectionType: Item.ID.self) { ids in
            let targets = store.items.filter { ids.contains($0.id) }
            if let path = targets.first?.path {
                Button("Reveal in Finder") { NSWorkspace.shared.activateFileViewerSelecting([path]) }
            }
            Button("Remove…", role: .destructive) { pendingRemoval = targets.filter { $0.removal.isSupported } }
                .disabled(!targets.contains { $0.removal.isSupported })
        } primaryAction: { ids in
            if let path = store.items.first(where: { ids.contains($0.id) })?.path {
                NSWorkspace.shared.activateFileViewerSelecting([path])
            }
        }
        .overlay {
            if store.items.isEmpty && !store.isWorking {
                ContentUnavailableView("Nothing found", systemImage: "tray",
                                       description: Text("No supported agent folders were found in your home folder."))
            } else if visible.isEmpty && !search.isEmpty {
                ContentUnavailableView.search(text: search)
            }
        }
    }

    // MARK: - Toolbar & dialogs

    @ToolbarContentBuilder
    private var toolbar: some ToolbarContent {
        ToolbarItemGroup {
            if store.isWorking { ProgressView().controlSize(.small) }
            Button { Task { await store.refresh() } } label: { Label("Refresh", systemImage: "arrow.clockwise") }
                .help("Scan again (⌘R)")
            Button(role: .destructive) {
                pendingRemoval = selectedItems.filter { $0.removal.isSupported }
            } label: {
                Label("Remove", systemImage: "trash")
            }
            .disabled(!selectedItems.contains { $0.removal.isSupported })
            .help("Remove the selected items")
            Button { showInspector.toggle() } label: { Label("Inspector", systemImage: "sidebar.right") }
        }
    }

    private var removalTitle: String {
        pendingRemoval.count == 1 ? "Remove \(pendingRemoval[0].name)?" : "Remove \(pendingRemoval.count) items?"
    }

    private var removalBinding: Binding<Bool> {
        Binding(get: { !pendingRemoval.isEmpty }, set: { if !$0 { pendingRemoval = [] } })
    }

    private var errorBinding: Binding<Bool> {
        Binding(get: { store.errorMessage != nil }, set: { if !$0 { store.errorMessage = nil } })
    }
}

/// Takes a closure instead of reading the Store from the environment: Table cells on macOS don't get it.
struct StatusCell: View {
    let item: Item
    let setEnabled: (Bool) -> Void

    var body: some View {
        if let enabled = item.enabled {
            Toggle("Enabled", isOn: Binding(get: { enabled }, set: setEnabled))
                .toggleStyle(.switch)
                .controlSize(.mini)
                .labelsHidden()
                .help(enabled ? "Enabled" : "Disabled")
        } else if item.kind == .leftover {
            Text("Reclaimable").foregroundStyle(.orange)
        } else if item.kind == .marketplace, item.note != nil {
            Text("Unused").foregroundStyle(.secondary)
        } else {
            Text("")
        }
    }
}

struct SummaryBar: View {
    let items: [Item]
    let cleanLeftovers: () -> Void

    var body: some View {
        let bytes = items.reduce(Int64(0)) { $0 + $1.bytes }
        let tokens = items.reduce(0) { $0 + $1.sessionTokens }
        let leftovers = items.filter { $0.kind == .leftover }
        HStack(spacing: 20) {
            stat("\(items.count)", "items")
            stat(Format.bytes(bytes), "on disk")
            stat("~" + Format.tokens(tokens), "tokens loaded per session")
                .help("Skill, agent and command descriptions plus always-loaded memory, for enabled items. Estimated at 4 characters per token.")
            Spacer()
            if !leftovers.isEmpty {
                Button(action: cleanLeftovers) {
                    Label("Clean up \(Format.bytes(leftovers.reduce(0) { $0 + $1.bytes }))", systemImage: "sparkles")
                }
                .buttonStyle(.borderedProminent)
                .tint(.orange)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }

    private func stat(_ value: String, _ label: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(value).font(.title3.weight(.semibold)).monospacedDigit()
            Text(label).font(.caption).foregroundStyle(.secondary)
        }
    }
}

enum Format {
    static func bytes(_ value: Int64) -> String {
        ByteCountFormatter.string(fromByteCount: value, countStyle: .file)
    }

    static func tokens(_ value: Int) -> String {
        value.formatted(.number.notation(.compactName))
    }
}
