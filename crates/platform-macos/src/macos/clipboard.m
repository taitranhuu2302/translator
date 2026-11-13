#import "clipboard.h"

#import <AppKit/AppKit.h>

// The popup's Copy button. Writing through AppKit keeps the behaviour identical
// to a ⌘C in a native app and avoids relying on the web view's clipboard
// permission, which differs between signed builds.
bool tl_write_clipboard(const char *text) {
    if (text == NULL) {
        return false;
    }

    NSString *value = [NSString stringWithUTF8String:text];
    if (value == nil) {
        return false;
    }

    NSPasteboard *pasteboard = [NSPasteboard generalPasteboard];
    [pasteboard clearContents];
    return [pasteboard setString:value forType:NSPasteboardTypeString];
}
