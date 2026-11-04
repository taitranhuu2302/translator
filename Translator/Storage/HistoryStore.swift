import Foundation

struct HistoryEntry: Identifiable, Codable {
    let id: UUID
    let createdAt: Date
    let source: String
    let target: String
    let sourceLanguage: String?
    let targetLanguage: String
    let provider: String
    let action: Action

    enum Action: String, Codable {
        case translate
        case improve
        case replace
    }
}

/// Local, searchable history. Rows older than 30 days are purged.
///
/// Backed by SQLite in a later step; the interface is here so callers do not
/// depend on the storage engine.
final class HistoryStore {
    private(set) var entries: [HistoryEntry] = []

    func record(_ entry: HistoryEntry) {
        entries.append(entry)
    }

    func search(_ query: String) -> [HistoryEntry] {
        guard !query.isEmpty else { return entries }
        return entries.filter {
            $0.source.localizedCaseInsensitiveContains(query)
                || $0.target.localizedCaseInsensitiveContains(query)
        }
    }

    func purge(olderThan days: Int = 30) {
        let cutoff = Calendar.current.date(byAdding: .day, value: -days, to: Date()) ?? .distantPast
        entries.removeAll { $0.createdAt < cutoff }
    }
}
