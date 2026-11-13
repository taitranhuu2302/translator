#import <AVFoundation/AVFoundation.h>

#include <dispatch/dispatch.h>
#include <string.h>

#include "speech.h"

/*
 * Read Aloud speaks the Selection in the system voice. The synthesizer is
 * created and driven on the main thread — that is where AVFoundation expects its
 * run loop — while the engine calls in from a worker thread, so the utterance is
 * handed over with dispatch_async and the shortcut never waits for speech.
 */

/*
 * A fresh utterance is not spoken until the cancelled one has actually stopped.
 * AVSpeechSynthesizer drops a speakUtterance: issued while a cancellation is
 * still in flight, which is how a second press ends up silent. Holding the
 * newest utterance until the delegate reports the cancellation makes both
 * "press again while it is talking" and "press after it finished" work.
 */
@interface TLSpeechSpeaker : NSObject <AVSpeechSynthesizerDelegate>
@property(nonatomic, strong) AVSpeechSynthesizer *synthesizer;
@property(nonatomic, strong) AVSpeechUtterance *pending;
@end

@implementation TLSpeechSpeaker

- (instancetype)init {
    if ((self = [super init])) {
        _synthesizer = [[AVSpeechSynthesizer alloc] init];
        _synthesizer.delegate = self;
    }
    return self;
}

- (void)speak:(AVSpeechUtterance *)utterance {
    if (self.synthesizer.isSpeaking) {
        /* Only the newest press matters; an older pending utterance is dropped. */
        self.pending = utterance;
        [self.synthesizer stopSpeakingAtBoundary:AVSpeechBoundaryImmediate];
        return;
    }
    self.pending = nil;
    [self.synthesizer speakUtterance:utterance];
}

- (void)speechSynthesizer:(AVSpeechSynthesizer *)synthesizer
    didCancelSpeechUtterance:(AVSpeechUtterance *)utterance {
    AVSpeechUtterance *next = self.pending;
    self.pending = nil;
    if (next) {
        [self.synthesizer speakUtterance:next];
    }
}

@end

static TLSpeechSpeaker *sharedSpeaker(void) {
    static TLSpeechSpeaker *speaker = nil;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
      speaker = [[TLSpeechSpeaker alloc] init];
    });
    return speaker;
}

/*
 * The system names voices by a full tag ("vi-VN", "en-US") while the engine asks
 * for a bare code ("vi"), so an exact match is tried first, then any voice whose
 * tag starts with the code, then the code's base language.
 */
static AVSpeechSynthesisVoice *voiceForLanguage(NSString *code) {
    AVSpeechSynthesisVoice *voice = [AVSpeechSynthesisVoice voiceWithLanguage:code];
    if (voice) {
        return voice;
    }

    NSString *prefix = [code stringByAppendingString:@"-"];
    for (AVSpeechSynthesisVoice *candidate in [AVSpeechSynthesisVoice speechVoices]) {
        if ([candidate.language hasPrefix:prefix]) {
            return candidate;
        }
    }

    NSRange separator = [code rangeOfString:@"-"];
    if (separator.location != NSNotFound) {
        return voiceForLanguage([code substringToIndex:separator.location]);
    }
    return nil;
}

int32_t tl_speak(const char *text, const char *language) {
    if (!text || !language) {
        return TL_SPEECH_UNAVAILABLE;
    }

    @autoreleasepool {
        NSString *code = [NSString stringWithUTF8String:language];
        NSString *string = [NSString stringWithUTF8String:text];
        if (code.length == 0 || !string) {
            return TL_SPEECH_UNAVAILABLE;
        }

        AVSpeechSynthesisVoice *voice = voiceForLanguage(code);
        if (!voice) {
            return TL_SPEECH_NO_VOICE;
        }

        AVSpeechUtterance *utterance = [AVSpeechUtterance speechUtteranceWithString:string];
        utterance.voice = voice;

        dispatch_async(dispatch_get_main_queue(), ^{
          [sharedSpeaker() speak:utterance];
        });
        return TL_SPEECH_OK;
    }
}
