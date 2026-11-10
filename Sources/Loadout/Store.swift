import LoadoutCore
import SwiftUI

enum SidebarSelection: Hashable {
    case all
    case kind(Kind)
    case agent(String)
}

@MainActor
@Observable
final class Store {
    private(set) var items: [Item] = []
    private(set) var isWorking = false
    var errorMessage: String?

    private let scanner = InventoryScanner()
    private let actions = Actions()

    /// Agents in the order of `AgentSpec.all`, only those found on this Mac.
    var agents: [String] {
        let found = Set(items.map(\.agent))
        return AgentSpec.all.map(\.name).filter(found.contains)
    }

    func items(for selection: SidebarSelection) -> [Item] {
        switch selection {
        case .all: items
        case .kind(let kind): items.filter { $0.kind == kind }
        case .agent(let agent): items.filter { $0.agent == agent }
        }
    }

    func refresh() async {
        isWorking = true
        let scanner = scanner
        items = await Task.detached(priority: .userInitiated) { scanner.scanAll() }.value
        isWorking = false
    }

    func remove(_ targets: [Item]) async {
        isWorking = true
        var failures: [String] = []
        for item in targets where item.removal.isSupported {
            do {
                try await actions.remove(item)
            } catch {
                failures.append("\(item.name): \(error.localizedDescription)")
            }
        }
        if !failures.isEmpty { errorMessage = failures.joined(separator: "\n\n") }
        await refresh()
    }

    func setEnabled(_ item: Item, _ enabled: Bool) async {
        isWorking = true
        do {
            try await actions.setEnabled(item, enabled)
        } catch {
            errorMessage = "\(item.name): \(error.localizedDescription)"
        }
        await refresh()
    }
}
