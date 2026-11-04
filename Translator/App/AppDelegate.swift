import AppKit
import Carbon.HIToolbox

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem?
    private var windowController: TranslateWindowController?
    private var quickPanel: QuickTranslatePanel?
    private var hotKeys: HotKeyManager?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        installStatusItem()
        installHotKeys()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    // MARK: - Menu bar

    private func installStatusItem() {
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = item.button {
            let image = NSImage(systemSymbolName: "character.bubble", accessibilityDescription: "Translator")
            image?.isTemplate = true
            button.image = image
            button.toolTip = "Translator"
        }

        let menu = NSMenu()
        let openItem = NSMenuItem(title: "Open Translator", action: #selector(showTranslateWindow), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Translator", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        item.menu = menu

        statusItem = item
    }

    // MARK: - Shortcuts

    private func installHotKeys() {
        let manager = HotKeyManager()
        let modifiers = UInt32(controlKey | optionKey)

        manager.register(keyCode: UInt32(kVK_ANSI_D), modifiers: modifiers) { [weak self] in
            self?.quickTranslate()
        }
        manager.register(keyCode: UInt32(kVK_ANSI_R), modifiers: modifiers) { [weak self] in
            self?.quickReplace()
        }
        manager.register(keyCode: UInt32(kVK_ANSI_S), modifiers: modifiers) { [weak self] in
            self?.readAloud()
        }
        manager.register(keyCode: UInt32(kVK_ANSI_E), modifiers: modifiers) { [weak self] in
            self?.showTranslateWindow()
        }

        hotKeys = manager
    }

    // MARK: - Actions

    @objc private func showTranslateWindow() {
        if windowController == nil {
            windowController = TranslateWindowController()
        }
        windowController?.showWindow(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    private func quickTranslate() {
        let panel = quickPanel ?? QuickTranslatePanel()
        quickPanel = panel
        panel.present(
            headline: "Quick Translate",
            detail: "Selection capture is not wired up yet."
        )
    }

    private func quickReplace() {
        NSSound.beep()
    }

    private func readAloud() {
        NSSound.beep()
    }
}
