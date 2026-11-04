import Foundation

/// A machine translation backend. The app picks one at runtime according to the
/// ladder recorded in `docs/adr/0003-translation-provider-ladder.md`.
protocol TranslationProvider {
    var name: String { get }
    func translate(_ text: String, from source: String?, to target: String) async throws -> String
}

struct TranslationOutcome {
    let text: String
    let provider: String
}

enum TranslationError: LocalizedError {
    case missingKey(String)
    case badResponse
    case quotaExceeded
    case unsupported

    var errorDescription: String? {
        switch self {
        case .missingKey(let provider):
            return "No API key is configured for \(provider)."
        case .badResponse:
            return "The translation service returned an unexpected response."
        case .quotaExceeded:
            return "The translation service's quota has been exceeded."
        case .unsupported:
            return "This request is not supported by the translation service."
        }
    }
}
