import AppKit
import SwiftUI

final class TranslateWindowController: NSWindowController {
    convenience init() {
        let window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 780, height: 520),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        window.title = "Translator"
        window.titlebarAppearsTransparent = true
        window.center()
        window.contentView = NSHostingView(rootView: TranslateWindowView())
        window.setFrameAutosaveName("TranslateWindow")
        self.init(window: window)
    }
}

struct TranslateWindowView: View {
    private enum Mode: String, CaseIterable, Identifiable {
        case translate = "Translate"
        case improve = "Improve"

        var id: String { rawValue }
    }

    @State private var mode: Mode = .translate
    @State private var input: String = ""
    @State private var result: String = ""

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            HStack(spacing: 0) {
                inputPane
                Divider()
                resultPane
            }
        }
        .background(DS.Palette.surface)
        .frame(minWidth: 640, minHeight: 420)
    }

    private var header: some View {
        HStack(spacing: DS.Spacing.m) {
            Picker("Mode", selection: $mode) {
                ForEach(Mode.allCases) { mode in
                    Text(mode.rawValue).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(width: 200)

            Spacer()

            Button {
                // Read Aloud is wired in a later step.
            } label: {
                Label("Read aloud", systemImage: "speaker.wave.2")
            }
            .buttonStyle(.borderless)
            .foregroundStyle(DS.Palette.subtleText)
            .disabled(input.isEmpty)

            Button(action: run) {
                Text(mode == .translate ? "Translate" : "Improve")
                    .font(DS.Typography.emphasized)
            }
            .keyboardShortcut(.return, modifiers: [])
            .disabled(input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        .padding(.horizontal, DS.Spacing.l)
        .padding(.vertical, DS.Spacing.m)
    }

    private var inputPane: some View {
        ZStack(alignment: .topLeading) {
            TextEditor(text: $input)
                .font(DS.Typography.body)
                .scrollContentBackground(.hidden)
                .padding(DS.Spacing.m)

            if input.isEmpty {
                Text("Paste or type text, then press ⏎")
                    .font(DS.Typography.body)
                    .foregroundStyle(DS.Palette.subtleText.opacity(0.7))
                    .padding(.horizontal, DS.Spacing.m + 5)
                    .padding(.vertical, DS.Spacing.m + 8)
                    .allowsHitTesting(false)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var resultPane: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: DS.Spacing.m) {
                if result.isEmpty {
                    Text("Translation appears here.")
                        .font(DS.Typography.body)
                        .foregroundStyle(DS.Palette.subtleText.opacity(0.7))
                } else {
                    Text(result)
                        .font(DS.Typography.reading)
                        .lineSpacing(6)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(DS.Spacing.l)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(DS.Palette.sunken.opacity(0.4))
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func run() {
        // Providers are wired in a later step.
        result = mode == .translate
            ? "Translation provider is not wired up yet."
            : "Improve is not wired up yet."
    }
}
