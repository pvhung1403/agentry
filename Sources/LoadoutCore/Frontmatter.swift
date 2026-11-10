import Foundation

enum Frontmatter {
    /// Minimal YAML frontmatter reader: top-level `key: value` pairs and indented block values
    /// (`|`, `>`), joined into one line. Enough for `name` / `description` in SKILL.md and agent files.
    static func parse(_ text: String) -> [String: String] {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: CharacterSet(charactersIn: "\r")) }
        guard lines.first?.trimmingCharacters(in: .whitespaces) == "---" else { return [:] }

        var out: [String: String] = [:]
        var blockKey: String?
        var block: [String] = []
        func flush() {
            if let key = blockKey { out[key] = block.joined(separator: " ") }
            blockKey = nil
            block = []
        }

        for line in lines.dropFirst() {
            if line.trimmingCharacters(in: .whitespaces) == "---" { break }
            if blockKey != nil, line.first == " " || line.first == "\t" {
                block.append(line.trimmingCharacters(in: .whitespaces))
                continue
            }
            flush()
            guard let colon = line.firstIndex(of: ":"), line.first != " ", line.first != "#" else { continue }
            let key = line[..<colon].trimmingCharacters(in: .whitespaces)
            var value = line[line.index(after: colon)...].trimmingCharacters(in: .whitespaces)
            if ["", "|", ">", "|-", ">-", "|+", ">+"].contains(value) {
                blockKey = key
                continue
            }
            if value.count >= 2, let quote = value.first, quote == "\"" || quote == "'", value.last == quote {
                value = String(value.dropFirst().dropLast())
            }
            out[key] = value
        }
        flush()
        return out
    }
}
