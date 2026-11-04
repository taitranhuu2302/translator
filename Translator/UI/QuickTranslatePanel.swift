import AppKit
import SwiftUI

/// The popup shown next to the cursor by Quick Translate.
///
/// It must never become the key window: if it did, the app whose text we are
/// about to replace would deactivate and the synthesized paste would land
/// here instead. That is why this is an `NSPanel` with `.nonactivatingPanel`.
final class QuickTranslatePanel: NSPanel {
    private let panelSize = NSSize(width: 420, height: 168)

    init() {
        super.init(
            contentRect: NSRect(origin: .zero, size: panelSize),
            styleMask: [.nonactivatingPanel, .borderless, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )

        isFloatingPanel = true
        level = .floating
        hidesOnDeactivate = false
        isMovableByWindowBackground = true
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .transient]
        backgroundColor = .clear
        isOpaque = false
        hasShadow = true
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }

    func present(headline: String, detail: String) {
        contentView = NSHostingView(rootView: QuickTranslateView(headline: headline, detail: detail))
        positionNearCursor()
        orderFrontRegardless()
    }

    private func positionNearCursor() {
        let mouse = NSEvent.mouseLocation
        let screen = NSScreen.screens.first { NSMouseInRect(mouse, $0.frame, false) } ?? NSScreen.main

        var origin = NSPoint(x: mouse.x + 14, y: mouse.y - panelSize.height - 14)

        if let visible = screen?.visibleFrame {
            origin.x = min(origin.x, visible.maxX - panelSize.width - 8)
            origin.x = max(origin.x, visible.minX + 8)
            origin.y = min(origin.y, visible.maxY - panelSize.height - 8)
            origin.y = max(origin.y, visible.minY + 8)
        }

        setFrame(NSRect(origin: origin, size: panelSize), display: true)
    }
}

private struct QuickTranslateView: View {
    let headline: String
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Spacing.s) {
            Text(headline)
                .font(DS.Typography.title)
                .foregroundStyle(.primary)

            Text(detail)
                .font(DS.Typography.body)
                .foregroundStyle(DS.Palette.subtleText)
                .fixedSize(horizontal: false, vertical: true)

            Spacer(minLength: 0)

            HStack(spacing: DS.Spacing.m) {
                hint("⏎", "Replace")
                hint("esc", "Dismiss")
                Spacer()
            }
        }
        .padding(DS.Spacing.l)
        .frame(width: 420, height: 168, alignment: .topLeading)
        .background(DS.Palette.raised)
        .clipShape(RoundedRectangle(cornerRadius: DS.Radius.large, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: DS.Radius.large, style: .continuous)
                .strokeBorder(DS.Palette.border, lineWidth: 1)
        )
    }

    private func hint(_ key: String, _ label: String) -> some View {
        HStack(spacing: DS.Spacing.xs) {
            Text(key)
                .font(DS.Typography.caption)
                .padding(.horizontal, DS.Spacing.xs + 2)
                .padding(.vertical, 1)
                .background(DS.Palette.sunken)
                .clipShape(RoundedRectangle(cornerRadius: DS.Radius.small, style: .continuous))
            Text(label)
                .font(DS.Typography.caption)
                .foregroundStyle(DS.Palette.subtleText)
        }
    }
}
