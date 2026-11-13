#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <Carbon/Carbon.h>

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
static NSString *const kSecureTextFieldSubrole = @"AXSecureTextField";
static NSString *const kManualAccessibilityAttribute = @"AXManualAccessibility";

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

static void synthesizeCommandC(void) {
    CGEventSourceRef source = CGEventSourceCreate(kCGEventSourceStateCombinedSessionState);
    CGEventRef down = CGEventCreateKeyboardEvent(source, (CGKeyCode)kVK_ANSI_C, true);
    CGEventRef up = CGEventCreateKeyboardEvent(source, (CGKeyCode)kVK_ANSI_C, false);
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

char *tl_read_selection(int32_t *status) {
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
        if (status) {
            *status = TL_CAPTURE_SECURE_INPUT;
        }
        return NULL;
    }

    AXUIElementRef element = focusedElement();
    if (element && focusedElementIsSecure(element)) {
        CFRelease(element);
        if (status) {
            *status = TL_CAPTURE_SECURE_INPUT;
        }
        return NULL;
    }

    NSString *text = element ? AXStringAttribute(element, kAXSelectedTextAttribute) : nil;
    if (element) {
        CFRelease(element);
    }

    if (text.length == 0) {
        text = selectionViaClipboard();
    }

    NSCharacterSet *blanks = [NSCharacterSet whitespaceAndNewlineCharacterSet];
    BOOL meaningful = text.length > 0 &&
                      [[text stringByTrimmingCharactersInSet:blanks] length] > 0;
    if (!meaningful) {
        if (status) {
            *status = TL_CAPTURE_NOTHING_SELECTED;
        }
        return NULL;
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
