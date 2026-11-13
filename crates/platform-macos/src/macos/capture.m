#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <Carbon/Carbon.h>

#include <dispatch/dispatch.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "capture.h"

/*
 * Accessibility first, clipboard as the fallback
 * (docs/adr/0002-accessibility-first-clipboard-fallback.md).
 *
 * The attribute the selected text lives in is compared by its documented value
 * rather than a generated constant, so a rename shows up here as an empty read
 * rather than a compile error somewhere else.
 */
static NSString *const kSelectedTextAttribute = @"AXSelectedText";
static NSString *const kSelectedTextRangeAttribute = @"AXSelectedTextRange";
static NSString *const kValueAttribute = @"AXValue";
static NSString *const kSecureTextFieldSubrole = @"AXSecureTextField";
static NSString *const kManualAccessibilityAttribute = @"AXManualAccessibility";

/*
 * Accessibility is synchronous cross-process IPC with a 6 s default timeout: a
 * hung target would beachball the caller. Shipping tools bound it to a quarter
 * to half a second before touching the target.
 */
static const float kAXMessagingTimeout = 0.5f;

/*
 * How long the target app gets to apply an Accessibility write before it counts
 * as "said yes and did nothing" (Word returns success and changes nothing). The
 * delays are the backoff the platform notes recommend: quick at first, because a
 * native text control updates at once, then patient for the async web stacks.
 */
static const useconds_t kAXWriteBackoff[] = {10000, 25000, 50000, 100000, 250000}; /* 10/25/50/100/250 ms */

/* How long the target app gets to service a synthesized paste before the saved
 * clipboard is put back. The read path waits 50 ms; a paste inserts text and is
 * therefore slower, and restoring before it is serviced makes the app paste the
 * user's old clipboard instead. */
static const useconds_t kPasteSettle = 120000; /* 120 ms */

/*
 * The target of the last successful read, retained so Replace writes where the
 * Selection came from — the Quick Translate panel holds the keyboard by the time
 * Enter is pressed, so the focused element then may be ours. `gRetainedRange` is
 * the Selection's location in the source app, in UTF-16 code units, or
 * kCFNotFound when the app does not expose it.
 */
static AXUIElementRef gRetainedElement = NULL;
static CFRange gRetainedRange = {-1, 0};
static NSLock *gRetainedLock = nil;

static NSLock *retainedLock(void) {
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        gRetainedLock = [[NSLock alloc] init];
    });
    return gRetainedLock;
}

static void retainTarget(AXUIElementRef element, CFRange range) {
    [retainedLock() lock];
    if (gRetainedElement) {
        CFRelease(gRetainedElement);
    }
    gRetainedElement = element ? (AXUIElementRef)CFRetain(element) : NULL;
    gRetainedRange = element ? range : CFRangeMake(kCFNotFound, 0);
    [retainedLock() unlock];
}

static void clearRetainedTarget(void) {
    retainTarget(NULL, CFRangeMake(kCFNotFound, 0));
}

/* Answers a +1 reference, which the caller releases, or NULL when none is held. */
static AXUIElementRef copyRetainedTarget(CFRange *range) {
    [retainedLock() lock];
    AXUIElementRef element = gRetainedElement ? (AXUIElementRef)CFRetain(gRetainedElement) : NULL;
    if (range) {
        *range = gRetainedRange;
    }
    [retainedLock() unlock];
    return element;
}

/*
 * Every Accessibility call runs on this one serial queue. AX messaging is
 * synchronous IPC into another process, and shipping tools that call it from
 * several threads report crashes; one dedicated thread is the safe pattern. The
 * queue also keeps a Replace from interleaving with a capture reading the same
 * target. It is never the main thread.
 */
static dispatch_queue_t accessibilityQueue(void) {
    static dispatch_queue_t queue;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        queue = dispatch_queue_create("com.translator.accessibility", DISPATCH_QUEUE_SERIAL);
    });
    return queue;
}

/*
 * Bound the time the target app gets to answer. Without it the default is 6 s,
 * which reads to the user as a beachball. Set on the application element, as the
 * platform notes describe; the system-wide element is never touched because
 * setting it there would change this process's default for every app.
 */
static void boundMessagingTimeout(AXUIElementRef element) {
    if (!element) {
        return;
    }
    pid_t pid = 0;
    if (AXUIElementGetPid(element, &pid) != kAXErrorSuccess) {
        return;
    }
    AXUIElementRef application = AXUIElementCreateApplication(pid);
    if (!application) {
        return;
    }
    AXUIElementSetMessagingTimeout(application, kAXMessagingTimeout);
    CFRelease(application);
}

/* Unique enough that a real selection can never be mistaken for it. */
static NSString *const kClipboardMarker = @"⁣TranslatorSelectionMarker⁣";

/* How long the target app gets to answer a synthesized copy. The clipboard path is
 * expected to cost 100–200 ms (ADR-0002), so 300 ms plus a short settle is the cap. */
static const useconds_t kCopyPollInterval = 5000; /* 5 ms */
static const NSTimeInterval kCopyTimeout = 0.3;
static const NSTimeInterval kCopySettle = 0.05;

static NSString *AXStringAttribute(AXUIElementRef element, CFStringRef attribute) {
    if (!element) {
        return nil;
    }
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element, attribute, &value) != kAXErrorSuccess) {
        return nil;
    }
    NSString *result = nil;
    if (value && CFGetTypeID(value) == CFStringGetTypeID()) {
        result = [(__bridge NSString *)value copy];
    }
    if (value) {
        CFRelease(value);
    }
    return result;
}

/*
 * The focused element belongs to the focused app, not to the system-wide element.
 * Electron apps (VS Code, Slack) expose nothing until AXManualAccessibility is set,
 * so the attribute is forced before asking; apps that do not know it return an
 * error that is harmless here.
 */
static AXUIElementRef focusedElement(void) {
    AXUIElementRef system = AXUIElementCreateSystemWide();
    CFTypeRef application = NULL;
    AXError error = AXUIElementCopyAttributeValue(
        system, kAXFocusedApplicationAttribute, &application);
    CFRelease(system);
    if (error != kAXErrorSuccess || !application) {
        if (application) {
            CFRelease(application);
        }
        return NULL;
    }

    AXUIElementSetAttributeValue((AXUIElementRef)application,
                                 (__bridge CFStringRef)kManualAccessibilityAttribute,
                                 kCFBooleanTrue);

    CFTypeRef element = NULL;
    error = AXUIElementCopyAttributeValue((AXUIElementRef)application,
                                          kAXFocusedUIElementAttribute, &element);
    CFRelease(application);
    if (error != kAXErrorSuccess || !element) {
        if (element) {
            CFRelease(element);
        }
        return NULL;
    }
    return (AXUIElementRef)element;
}

static BOOL focusedElementIsSecure(AXUIElementRef element) {
    NSString *subrole = AXStringAttribute(element, kAXSubroleAttribute);
    return [subrole isEqualToString:kSecureTextFieldSubrole];
}

/* Where the Selection starts, in UTF-16 code units, or kCFNotFound when the app
 * does not expose the attribute (browsers and terminals often do not). */
static CFRange selectedTextRange(AXUIElementRef element) {
    CFRange range = CFRangeMake(kCFNotFound, 0);
    if (!element) {
        return range;
    }
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element,
                                      (__bridge CFStringRef)kSelectedTextRangeAttribute,
                                      &value) == kAXErrorSuccess &&
        value) {
        if (CFGetTypeID(value) == AXValueGetTypeID()) {
            AXValueGetValue((AXValueRef)value, kAXValueCFRangeType, &range);
        }
        CFRelease(value);
    }
    return range;
}

/* Put the Selection over exactly the inserted text rather than leaving it longer
 * than the translation. The length is UTF-16 code units, never bytes. */
static void setSelectedRange(AXUIElementRef element, CFIndex location, size_t utf16Length) {
    if (!element || location == kCFNotFound) {
        return;
    }
    CFRange range = CFRangeMake(location, (CFIndex)utf16Length);
    AXValueRef value = AXValueCreate(kAXValueCFRangeType, &range);
    if (!value) {
        return;
    }
    AXUIElementSetAttributeValue(element,
                                 (__bridge CFStringRef)kSelectedTextRangeAttribute,
                                 value);
    CFRelease(value);
}

/*
 * Select exactly the text just inserted, from the range the app reports *now*.
 * Computing `(oldLocation, insertedLength)` blind overshoots: web stacks update
 * asynchronously, Chromium has been seen to mis-index, and after a paste the app
 * may already have moved the caret. A collapsed caret just past the inserted
 * text is the one shape that says where the new text landed; anything else is
 * left alone rather than selecting the wrong span.
 */
static void selectInsertedText(AXUIElementRef element, size_t utf16Length) {
    if (!element || utf16Length == 0) {
        return;
    }
    CFRange current = selectedTextRange(element);
    if (current.location == kCFNotFound) {
        /* The app does not expose a range (browsers usually do not). Leaving the
         * caret after the text is acceptable; selecting a guess is not. */
        return;
    }
    if (current.length == (CFIndex)utf16Length) {
        return; /* Already exactly the inserted text. */
    }
    if (current.length != 0 || current.location < (CFIndex)utf16Length) {
        return; /* A span of another size: do not guess. */
    }
    setSelectedRange(element, current.location - (CFIndex)utf16Length, utf16Length);
}

static NSArray<NSDictionary<NSString *, NSData *> *> *savePasteboard(NSPasteboard *pasteboard) {
    NSArray<NSPasteboardItem *> *items =
        [pasteboard readObjectsForClasses:@[ [NSPasteboardItem class] ] options:@{}];
    NSMutableArray<NSDictionary<NSString *, NSData *> *> *saved = [NSMutableArray array];
    for (NSPasteboardItem *item in items) {
        NSMutableDictionary<NSString *, NSData *> *contents = [NSMutableDictionary dictionary];
        for (NSPasteboardType type in item.types) {
            NSData *data = [item dataForType:type];
            if (data) {
                contents[type] = data;
            }
        }
        if (contents.count > 0) {
            [saved addObject:contents];
        }
    }
    return saved;
}

static void restorePasteboard(NSPasteboard *pasteboard,
                              NSArray<NSDictionary<NSString *, NSData *> *> *saved) {
    [pasteboard clearContents];
    if (saved.count == 0) {
        return;
    }
    NSMutableArray<NSPasteboardItem *> *items = [NSMutableArray array];
    for (NSDictionary<NSString *, NSData *> *contents in saved) {
        NSPasteboardItem *item = [[NSPasteboardItem alloc] init];
        for (NSString *type in contents) {
            [item setData:contents[type] forType:type];
        }
        [items addObject:item];
    }
    [pasteboard writeObjects:items];
}

static void synthesizeCommandKey(CGKeyCode key) {
    CGEventSourceRef source = CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
    CGEventRef down = CGEventCreateKeyboardEvent(source, key, true);
    CGEventRef up = CGEventCreateKeyboardEvent(source, key, false);
    if (down) {
        CGEventSetFlags(down, kCGEventFlagMaskCommand);
        CGEventPost(kCGHIDEventTap, down);
        CFRelease(down);
    }
    if (up) {
        CGEventSetFlags(up, kCGEventFlagMaskCommand);
        CGEventPost(kCGHIDEventTap, up);
        CFRelease(up);
    }
    if (source) {
        CFRelease(source);
    }
}

static void synthesizeCommandC(void) {
    synthesizeCommandKey((CGKeyCode)kVK_ANSI_C);
}

/*
 * The fallback every browser, terminal and Electron app takes: put a marker on the
 * clipboard, ask the focused app for a copy, read what landed, then put the user's
 * clipboard back — but only if nothing else wrote to it since our read, so a copy
 * the user made in the meantime is never clobbered.
 */
static NSString *selectionViaClipboard(void) {
    NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
    NSArray<NSDictionary<NSString *, NSData *> *> *saved = savePasteboard(pasteboard);

    [pasteboard clearContents];
    [pasteboard setString:kClipboardMarker forType:NSPasteboardTypeString];
    NSInteger markerCount = pasteboard.changeCount;

    synthesizeCommandC();

    NSInteger current = markerCount;
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:kCopyTimeout];
    while (current == markerCount && deadline.timeIntervalSinceNow > 0) {
        usleep(kCopyPollInterval);
        current = pasteboard.changeCount;
    }

    if (current != markerCount) {
        NSDate *settle = [NSDate dateWithTimeIntervalSinceNow:kCopySettle];
        while (settle.timeIntervalSinceNow > 0) {
            usleep(kCopyPollInterval);
            if (pasteboard.changeCount != current) {
                current = pasteboard.changeCount;
            }
        }
    }

    NSString *text = nil;
    if (current != markerCount) {
        text = [pasteboard stringForType:NSPasteboardTypeString];
        if ([text isEqualToString:kClipboardMarker]) {
            text = nil;
        }
    }

    if (pasteboard.changeCount == current) {
        restorePasteboard(pasteboard, saved);
    }
    return text;
}

static char *readSelectionLocked(int32_t *status) {
    if (status) {
        *status = TL_CAPTURE_UNAVAILABLE;
    }

    if (!AXIsProcessTrusted()) {
        if (status) {
            *status = TL_CAPTURE_NO_ACCESSIBILITY;
        }
        return NULL;
    }

    /* Refuse before reading anything: secure input means a password field owns the
     * keystrokes, and its contents must never be copied, logged or sent anywhere. */
    if (IsSecureEventInputEnabled()) {
        clearRetainedTarget();
        if (status) {
            *status = TL_CAPTURE_SECURE_INPUT;
        }
        return NULL;
    }

    AXUIElementRef element = focusedElement();
    boundMessagingTimeout(element);
    if (element && focusedElementIsSecure(element)) {
        clearRetainedTarget();
        CFRelease(element);
        if (status) {
            *status = TL_CAPTURE_SECURE_INPUT;
        }
        return NULL;
    }

    NSString *text = element ? AXStringAttribute(element, kAXSelectedTextAttribute) : nil;
    CFRange range = element ? selectedTextRange(element) : CFRangeMake(kCFNotFound, 0);

    if (text.length == 0) {
        text = selectionViaClipboard();
    }

    NSCharacterSet *blanks = [NSCharacterSet whitespaceAndNewlineCharacterSet];
    BOOL meaningful = text.length > 0 &&
                      [[text stringByTrimmingCharactersInSet:blanks] length] > 0;
    if (!meaningful) {
        clearRetainedTarget();
        if (element) {
            CFRelease(element);
        }
        if (status) {
            *status = TL_CAPTURE_NOTHING_SELECTED;
        }
        return NULL;
    }

    /* Keep where this came from, so Enter in the popup writes here rather than
     * into whatever happens to be focused by then. */
    retainTarget(element, range);
    if (element) {
        CFRelease(element);
    }

    const char *utf8 = text.UTF8String;
    char *copy = utf8 ? strdup(utf8) : NULL;
    if (!copy) {
        if (status) {
            *status = TL_CAPTURE_UNAVAILABLE;
        }
        return NULL;
    }
    if (status) {
        *status = TL_CAPTURE_OK;
    }
    return copy;
}

/* Capture and Replace share one serial Accessibility thread, so a read can never
 * interleave with a write on the same target. */
char *tl_read_selection(int32_t *status) {
    __block char *text = NULL;
    dispatch_sync(accessibilityQueue(), ^{
        text = readSelectionLocked(status);
    });
    return text;
}

/*
 * True when the element belongs to the app in front. The paste fallback can only
 * reach the frontmost app, so pasting while another app is in front would
 * overwrite something the user did not select. When the pid cannot be read the
 * answer is "no": a visible failure beats a write in the wrong place.
 */
static bool appOwnsTarget(AXUIElementRef element) {
    pid_t pid = 0;
    if (AXUIElementGetPid(element, &pid) != kAXErrorSuccess) {
        return false;
    }
    NSRunningApplication *frontmost = [[NSWorkspace sharedWorkspace] frontmostApplication];
    return frontmost != nil && frontmost.processIdentifier == pid;
}

/*
 * The Accessibility write: attempt it, then prove it landed.
 *
 * `AXUIElementIsAttributeSettable` is advisory at best — Chromium reports NO and
 * accepts, editors that own their model report YES and discard, and Word returns
 * success while changing nothing — so it is not a gate. Neither is the return
 * code. The reliable witness is `kAXValueAttribute` (the element's whole text)
 * changing; `AXSelectedText` reads back empty once the selection collapses, so
 * it proves nothing. When `AXValue` is unreadable (WebKit), the write cannot be
 * verified and is treated as not accepted, which sends the caller to the paste
 * path — the route every browser and terminal actually takes.
 */
static bool insertThroughAccessibility(AXUIElementRef element,
                                       NSString *text,
                                       size_t utf16Length) {
    if (!element) {
        return false;
    }
    NSString *before = AXStringAttribute(element, (__bridge CFStringRef)kValueAttribute);
    if (before == nil) {
        return false;
    }
    CFRange beforeRange = selectedTextRange(element);

    if (AXUIElementSetAttributeValue(element,
                                     (__bridge CFStringRef)kSelectedTextAttribute,
                                     (__bridge CFStringRef)text) != kAXErrorSuccess) {
        return false;
    }

    size_t attempts = sizeof(kAXWriteBackoff) / sizeof(kAXWriteBackoff[0]);
    for (size_t attempt = 0; attempt < attempts; attempt++) {
        usleep(kAXWriteBackoff[attempt]);
        NSString *after = AXStringAttribute(element, (__bridge CFStringRef)kValueAttribute);
        if (after != nil && ![after isEqualToString:before]) {
            selectInsertedText(element, utf16Length);
            return true;
        }
    }

    /*
     * One write legitimately leaves `AXValue` unchanged: replacing text with
     * itself. It is still distinguishable from an app that silently ignored the
     * write, because a real replace collapses the caret to just past the
     * inserted text while an ignored one leaves the selection untouched. With
     * neither a changed value nor a moved caret the write counts as refused, so
     * Replace reports a failure rather than claiming a success it cannot see.
     */
    if (beforeRange.location != kCFNotFound) {
        CFRange afterRange = selectedTextRange(element);
        if (afterRange.length == 0 &&
            afterRange.location == beforeRange.location + (CFIndex)utf16Length) {
            return true;
        }
    }
    return false;
}

/*
 * The fallback every browser, terminal and Electron app takes: put the
 * translation on the clipboard, paste it over the Selection with a synthesized
 * ⌘V, then put the user's clipboard back — but only if nothing else wrote to it
 * in the meantime. `changeCount` is the only ownership token: it moves on our
 * write and on anyone else's, so a user copy is never clobbered by the restore.
 */
static bool pasteOverSelection(AXUIElementRef element,
                               NSString *text,
                               size_t utf16Length) {
    NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
    NSArray<NSDictionary<NSString *, NSData *> *> *saved = savePasteboard(pasteboard);

    [pasteboard clearContents];
    if (![pasteboard setString:text forType:NSPasteboardTypeString]) {
        restorePasteboard(pasteboard, saved);
        return false;
    }
    NSInteger ours = pasteboard.changeCount;

    synthesizeCommandKey((CGKeyCode)kVK_ANSI_V);

    // The paste is asynchronous in the target app: give it time to read the
    // clipboard before the saved contents go back.
    usleep(kPasteSettle);
    if (pasteboard.changeCount == ours) {
        restorePasteboard(pasteboard, saved);
    }

    // The app collapses the caret after a paste. Select exactly the text that
    // landed, worked out from the range the app reports now rather than from a
    // location remembered before the paste; apps that expose no range keep the
    // caret after the text, which is acceptable.
    selectInsertedText(element, utf16Length);
    return true;
}

bool tl_replace_selection(const char *text_utf8, size_t utf16_length, int32_t *status) {
    if (status) {
        *status = TL_CAPTURE_UNAVAILABLE;
    }
    if (!text_utf8) {
        return false;
    }
    NSString *text = [NSString stringWithUTF8String:text_utf8];
    if (text == nil) {
        return false;
    }

    __block int32_t result = TL_CAPTURE_UNAVAILABLE;
    __block bool replaced = false;

    dispatch_sync(accessibilityQueue(), ^{
        if (!AXIsProcessTrusted()) {
            result = TL_CAPTURE_NO_ACCESSIBILITY;
            return;
        }

        /* Refuse before anything is written: with secure input active the
         * keystrokes belong to a password field and nothing may land there. This
         * also gates the paste path, whose synthesized ⌘V would otherwise be
         * delivered to whatever owns secure input. */
        if (IsSecureEventInputEnabled()) {
            result = TL_CAPTURE_SECURE_INPUT;
            return;
        }

        AXUIElementRef target = copyRetainedTarget(NULL);
        if (!target) {
            /* Only when no read retained a target: fall back to what is focused
             * now. With nothing focused there is nowhere to write. */
            target = focusedElement();
            if (!target) {
                result = TL_CAPTURE_NOTHING_TO_REPLACE;
                return;
            }
        }
        boundMessagingTimeout(target);

        /* Never touch our own app's elements from this thread: AppKit objects
         * must be messaged on the main thread, and a Replace aimed at our panel
         * means the shell hid it too late to reach the source app. */
        pid_t pid = 0;
        if (AXUIElementGetPid(target, &pid) == kAXErrorSuccess && pid == getpid()) {
            CFRelease(target);
            result = TL_CAPTURE_NOT_WRITABLE;
            return;
        }

        /* Password fields expose no AXSelectedText at all, so refuse them before
         * the attempt rather than discovering it afterwards. */
        if (focusedElementIsSecure(target)) {
            CFRelease(target);
            result = TL_CAPTURE_SECURE_INPUT;
            return;
        }

        if (insertThroughAccessibility(target, text, utf16_length)) {
            replaced = true;
        } else if (appOwnsTarget(target)) {
            /* The paste only reaches the frontmost app; when the target app is not
             * in front, the fallback would overwrite something the user did not
             * select, so it is refused instead. */
            replaced = pasteOverSelection(target, text, utf16_length);
        }
        CFRelease(target);
        result = replaced ? TL_CAPTURE_OK : TL_CAPTURE_NOT_WRITABLE;
    });

    if (status) {
        *status = result;
    }
    return replaced;
}

bool tl_accessibility_granted(void) {
    return AXIsProcessTrusted();
}

bool tl_request_accessibility(void) {
    NSDictionary *options = @{(__bridge NSString *)kAXTrustedCheckOptionPrompt : @YES};
    return AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);
}

bool tl_open_accessibility_settings(void) {
    NSURL *url = [NSURL URLWithString:@"x-apple.systempreferences:"
                                       @"com.apple.preference.security?Privacy_Accessibility"];
    return [[NSWorkspace sharedWorkspace] openURL:url];
}

void tl_string_free(char *text) {
    free(text);
}
