import AppKit
import Carbon.HIToolbox

/// Registers system-wide hotkeys through Carbon's `RegisterEventHotKey`.
///
/// Carbon is still the only way to receive a global shortcut without an
/// Input Monitoring permission prompt; the modern alternatives all need an
/// event tap.
final class HotKeyManager {
    private var handlers: [UInt32: () -> Void] = [:]
    private var references: [EventHotKeyRef?] = []
    private var nextID: UInt32 = 1
    private var eventHandler: EventHandlerRef?

    init() {
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )

        InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, context in
                guard let event, let context else { return noErr }
                let manager = Unmanaged<HotKeyManager>.fromOpaque(context).takeUnretainedValue()

                var hotKeyID = EventHotKeyID()
                let status = GetEventParameter(
                    event,
                    EventParamName(kEventParamDirectObject),
                    EventParamType(typeEventHotKeyID),
                    nil,
                    MemoryLayout<EventHotKeyID>.size,
                    nil,
                    &hotKeyID
                )

                if status == noErr, let handler = manager.handlers[hotKeyID.id] {
                    handler()
                }
                return noErr
            },
            1,
            &eventType,
            Unmanaged.passUnretained(self).toOpaque(),
            &eventHandler
        )
    }

    @discardableResult
    func register(keyCode: UInt32, modifiers: UInt32, action: @escaping () -> Void) -> Bool {
        let id = nextID
        nextID += 1

        var reference: EventHotKeyRef?
        let hotKeyID = EventHotKeyID(signature: OSType(0x5452_4E53), id: id)
        let status = RegisterEventHotKey(
            keyCode,
            modifiers,
            hotKeyID,
            GetApplicationEventTarget(),
            0,
            &reference
        )

        guard status == noErr else { return false }

        handlers[id] = action
        references.append(reference)
        return true
    }

    deinit {
        for reference in references {
            if let reference { UnregisterEventHotKey(reference) }
        }
        if let eventHandler { RemoveEventHandler(eventHandler) }
    }
}
