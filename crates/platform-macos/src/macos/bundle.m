#import <Foundation/Foundation.h>

#include <string.h>

#include "bundle.h"

char *tl_bundle_identifier(void) {
    @autoreleasepool {
        NSString *identifier = [[NSBundle mainBundle] bundleIdentifier];
        if (identifier == nil) {
            return NULL;
        }
        return strdup([identifier UTF8String]);
    }
}
