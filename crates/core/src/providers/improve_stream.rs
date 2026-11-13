//! Pulling the two improved fields out of the model's JSON while it is still
//! being written.
//!
//! `json_schema` structured outputs cannot stream, so the model is asked for
//! `json_object` and its text is accumulated. Waiting for the closing brace would
//! make the pane a spinner again, so this scanner walks the document as it
//! arrives and reports the decoded text of `corrected` and `better` as soon as it
//! is final.
//!
//! It is a scanner, not a JSON parser: it only needs the two keys and their
//! string values, in either order. Anything else — numbers, arrays, nested
//! objects, duplicates — is left to the strict `serde_json` parse that decides
//! whether the answer is valid at all. Its one hard rule is that a fragment must
//! never produce garbage: an escape or a UTF-8 code point split across two feeds
//! waits for the rest instead of emitting half a character.

use crate::model::{ImproveDelta, ImprovePart};

const CORRECTED_KEY: &str = "\"corrected\"";
const BETTER_KEY: &str = "\"better\"";
/// The longest key, minus one: a tail this long could still become a key.
const KEY_TAIL: usize = CORRECTED_KEY.len() - 1;

#[derive(Default)]
pub struct ImprovementStream {
    /// Bytes received but not yet a complete UTF-8 code point.
    undecoded: Vec<u8>,
    /// Decoded document text not yet consumed by the scanner.
    buffer: String,
    /// Bytes of `buffer` already consumed. Reset to zero when the buffer is
    /// trimmed at the end of each feed.
    position: usize,
    state: State,
    escape: Escape,
}

#[derive(Default)]
enum State {
    #[default]
    Seeking,
    /// The key was found; whitespace, then a colon, is expected.
    KeyColon(ImprovePart),
    /// The colon was found; whitespace, then the opening quote, is expected.
    ValueStart(ImprovePart),
    /// Inside the field's string value.
    InValue(ImprovePart),
}

#[derive(Default)]
enum Escape {
    #[default]
    Normal,
    AfterBackslash,
    Unicode {
        digits: u8,
        value: u32,
    },
    /// A high surrogate was decoded; a following `\uXXXX` low surrogate is
    /// expected. `saw_slash` tracks how much of the `\u` prefix has arrived.
    ExpectLowSurrogate {
        high: u32,
        saw_slash: bool,
    },
    LowSurrogate {
        high: u32,
        digits: u8,
        value: u32,
    },
}

/// Text decoded during one feed, coalesced into runs of the same part. Keeping
/// the order the characters were decoded in means a document that writes
/// `better` first streams `better` first.
#[derive(Default)]
struct Emissions {
    deltas: Vec<ImproveDelta>,
}

impl Emissions {
    fn push(&mut self, part: ImprovePart, character: char) {
        if let Some(last) = self.deltas.last_mut() {
            if last.part == part {
                last.text.push(character);
                return;
            }
        }
        self.deltas.push(ImproveDelta {
            part,
            text: character.to_string(),
        });
    }

    fn emit(self, sink: &mut dyn FnMut(&ImproveDelta)) {
        for delta in self.deltas {
            sink(&delta);
        }
    }
}

impl ImprovementStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one fragment of the model's accumulated JSON. Each piece of decoded
    /// field text that became final calls `sink`, in document order.
    pub fn feed(&mut self, bytes: &[u8], sink: &mut dyn FnMut(&ImproveDelta)) {
        self.undecoded.extend_from_slice(bytes);
        // Decode only a complete UTF-8 prefix: a code point may be split across
        // two fragments, and must never be decoded half-way.
        let valid = match std::str::from_utf8(&self.undecoded) {
            Ok(_) => self.undecoded.len(),
            Err(error) => error.valid_up_to(),
        };
        if valid > 0 {
            let text = std::str::from_utf8(&self.undecoded[..valid])
                .expect("from_utf8 reported this prefix as valid");
            self.buffer.push_str(text);
            self.undecoded.drain(..valid);
        }

        let mut emissions = Emissions::default();
        let mut state = std::mem::take(&mut self.state);
        let mut escape = std::mem::take(&mut self.escape);
        let mut position = self.position;
        scan(
            &self.buffer,
            &mut position,
            &mut state,
            &mut escape,
            &mut emissions,
        );
        self.position = position;
        self.state = state;
        self.escape = escape;

        if self.position > 0 {
            self.buffer.drain(..self.position);
            self.position = 0;
        }
        emissions.emit(sink);
    }
}

fn scan(
    buffer: &str,
    position: &mut usize,
    state: &mut State,
    escape: &mut Escape,
    emissions: &mut Emissions,
) {
    loop {
        match state {
            State::Seeking => match next_key(&buffer[*position..]) {
                Some((offset, part, length)) => {
                    *position += offset + length;
                    *state = State::KeyColon(part);
                }
                None => {
                    // Keep only a tail that could still begin a key.
                    *position = keep_tail(buffer, *position);
                    return;
                }
            },
            State::KeyColon(part) => {
                let part = *part;
                let mut progressed = false;
                while *position < buffer.len() {
                    let character = character_at(buffer, *position);
                    if character.is_whitespace() {
                        *position += character.len_utf8();
                        progressed = true;
                        continue;
                    }
                    if character == ':' {
                        *position += 1;
                        *state = State::ValueStart(part);
                    } else {
                        // Not a key after all (perhaps a value quoting one).
                        *state = State::Seeking;
                    }
                    progressed = true;
                    break;
                }
                if !progressed || *position >= buffer.len() {
                    return;
                }
            }
            State::ValueStart(part) => {
                let part = *part;
                let mut progressed = false;
                while *position < buffer.len() {
                    let character = character_at(buffer, *position);
                    if character.is_whitespace() {
                        *position += character.len_utf8();
                        progressed = true;
                        continue;
                    }
                    if character == '"' {
                        *position += 1;
                        *state = State::InValue(part);
                        *escape = Escape::Normal;
                    } else {
                        // A non-string value; the strict parse will reject it.
                        *state = State::Seeking;
                    }
                    progressed = true;
                    break;
                }
                if !progressed || *position >= buffer.len() {
                    return;
                }
            }
            State::InValue(part) => {
                let part = *part;
                let mut progressed = false;
                while *position < buffer.len() {
                    match escape {
                        Escape::Normal => {
                            let character = character_at(buffer, *position);
                            if character == '"' {
                                *position += 1;
                                *state = State::Seeking;
                                progressed = true;
                                break;
                            }
                            if character == '\\' {
                                *position += 1;
                                *escape = Escape::AfterBackslash;
                                progressed = true;
                                continue;
                            }
                            emissions.push(part, character);
                            *position += character.len_utf8();
                            progressed = true;
                        }
                        Escape::AfterBackslash => {
                            let character = character_at(buffer, *position);
                            let simple = match character {
                                '"' => Some('"'),
                                '\\' => Some('\\'),
                                '/' => Some('/'),
                                'b' => Some('\u{8}'),
                                'f' => Some('\u{c}'),
                                'n' => Some('\n'),
                                'r' => Some('\r'),
                                't' => Some('\t'),
                                _ => None,
                            };
                            if let Some(decoded) = simple {
                                emissions.push(part, decoded);
                                *position += 1;
                                *escape = Escape::Normal;
                            } else if character == 'u' {
                                *position += 1;
                                *escape = Escape::Unicode {
                                    digits: 0,
                                    value: 0,
                                };
                            } else {
                                // Invalid JSON keeps no character; the strict
                                // parse turns it into a typed error.
                                *position += 1;
                                *escape = Escape::Normal;
                            }
                            progressed = true;
                        }
                        Escape::Unicode { digits, value } => {
                            let character = character_at(buffer, *position);
                            match character.to_digit(16) {
                                Some(digit) => {
                                    *position += 1;
                                    let value = *value * 16 + digit;
                                    let digits = *digits + 1;
                                    if digits == 4 {
                                        finish_unicode(value, escape, emissions, part);
                                    } else {
                                        *escape = Escape::Unicode { digits, value };
                                    }
                                    progressed = true;
                                }
                                None => {
                                    // Fewer than four digits: an incomplete
                                    // escape. Wait for the rest of the fragment.
                                    return;
                                }
                            }
                        }
                        Escape::ExpectLowSurrogate { high, saw_slash } => {
                            let character = character_at(buffer, *position);
                            if *saw_slash {
                                if character == 'u' {
                                    *position += 1;
                                    *escape = Escape::LowSurrogate {
                                        high: *high,
                                        digits: 0,
                                        value: 0,
                                    };
                                    progressed = true;
                                } else {
                                    push_replacement(emissions, part);
                                    *escape = Escape::Normal;
                                }
                            } else if character == '\\' {
                                *position += 1;
                                *escape = Escape::ExpectLowSurrogate {
                                    high: *high,
                                    saw_slash: true,
                                };
                                progressed = true;
                            } else {
                                push_replacement(emissions, part);
                                *escape = Escape::Normal;
                            }
                        }
                        Escape::LowSurrogate {
                            high,
                            digits,
                            value,
                        } => {
                            let character = character_at(buffer, *position);
                            match character.to_digit(16) {
                                Some(digit) => {
                                    *position += 1;
                                    let value = *value * 16 + digit;
                                    let digits = *digits + 1;
                                    if digits == 4 {
                                        let high = *high;
                                        if (0xDC00..=0xDFFF).contains(&value) {
                                            let scalar = 0x1_0000
                                                + ((high - 0xD800) << 10)
                                                + (value - 0xDC00);
                                            if let Some(character) = char::from_u32(scalar) {
                                                emissions.push(part, character);
                                            } else {
                                                push_replacement(emissions, part);
                                            }
                                        } else {
                                            // A high surrogate not followed by a
                                            // low one is not a character.
                                            push_replacement(emissions, part);
                                        }
                                        *escape = Escape::Normal;
                                    } else {
                                        *escape = Escape::LowSurrogate {
                                            high: *high,
                                            digits,
                                            value,
                                        };
                                    }
                                    progressed = true;
                                }
                                None => return,
                            }
                        }
                    }
                }
                if !progressed || *position >= buffer.len() {
                    return;
                }
            }
        }
    }
}

/// A completed `\uXXXX`: either a character, half of a surrogate pair, or a
/// surrogate with no partner.
fn finish_unicode(value: u32, escape: &mut Escape, emissions: &mut Emissions, part: ImprovePart) {
    if (0xD800..=0xDBFF).contains(&value) {
        *escape = Escape::ExpectLowSurrogate {
            high: value,
            saw_slash: false,
        };
    } else if let Some(character) = char::from_u32(value) {
        emissions.push(part, character);
        *escape = Escape::Normal;
    } else {
        push_replacement(emissions, part);
        *escape = Escape::Normal;
    }
}

fn push_replacement(emissions: &mut Emissions, part: ImprovePart) {
    emissions.push(part, '\u{FFFD}');
}

fn character_at(buffer: &str, position: usize) -> char {
    buffer[position..]
        .chars()
        .next()
        .expect("a position inside the buffer")
}

/// The earliest `"corrected"` or `"better"` key in `rest`, with its byte offset
/// and length.
fn next_key(rest: &str) -> Option<(usize, ImprovePart, usize)> {
    let corrected = rest.find(CORRECTED_KEY);
    let better = rest.find(BETTER_KEY);
    match (corrected, better) {
        (Some(a), Some(b)) if a <= b => Some((a, ImprovePart::Corrected, CORRECTED_KEY.len())),
        (Some(_), Some(b)) => Some((b, ImprovePart::Better, BETTER_KEY.len())),
        (Some(a), None) => Some((a, ImprovePart::Corrected, CORRECTED_KEY.len())),
        (None, Some(b)) => Some((b, ImprovePart::Better, BETTER_KEY.len())),
        (None, None) => None,
    }
}

/// Drop everything that can no longer begin a key, keeping at most a partial key
/// at the tail. Offsets stay on character boundaries.
fn keep_tail(buffer: &str, position: usize) -> usize {
    let rest = &buffer[position..];
    let keep = rest.len().saturating_sub(KEY_TAIL);
    let mut from = position + keep;
    while from > position && !buffer.is_char_boundary(from) {
        from -= 1;
    }
    from
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One document with both escapes and literal non-ASCII text, `better`
    /// first so field order cannot matter.
    const DOCUMENT: &str = r#"{"better":"She said \"hi\" and \\ left.\nA better line: caféh","corrected":"He goé home."}"#;

    fn expected() -> (String, String) {
        (
            "He goé home.".to_owned(),
            "She said \"hi\" and \\ left.\nA better line: caféh".to_owned(),
        )
    }

    fn collect(fragments: &[&[u8]]) -> (String, String) {
        let mut stream = ImprovementStream::new();
        let mut corrected = String::new();
        let mut better = String::new();
        for fragment in fragments {
            stream.feed(fragment, &mut |delta| match delta.part {
                ImprovePart::Corrected => corrected.push_str(&delta.text),
                ImprovePart::Better => better.push_str(&delta.text),
            });
        }
        (corrected, better)
    }

    #[test]
    fn one_feed_reports_the_whole_document() {
        let (corrected, better) = collect(&[DOCUMENT.as_bytes()]);
        let (expected_corrected, expected_better) = expected();
        assert_eq!(corrected, expected_corrected);
        assert_eq!(better, expected_better);
    }

    #[test]
    fn every_byte_boundary_reconstructs_both_fields() {
        let bytes = DOCUMENT.as_bytes();
        let (expected_corrected, expected_better) = expected();
        for split in 0..=bytes.len() {
            let (corrected, better) = collect(&[&bytes[..split], &bytes[split..]]);
            assert_eq!(
                corrected, expected_corrected,
                "corrected differed when split at byte {split}"
            );
            assert_eq!(
                better, expected_better,
                "better differed when split at byte {split}"
            );
        }
    }

    #[test]
    fn one_byte_at_a_time_never_emits_a_split_character() {
        let bytes = DOCUMENT.as_bytes();
        let fragments: Vec<&[u8]> = bytes.chunks(1).collect();
        let (corrected, better) = collect(&fragments);
        let (expected_corrected, expected_better) = expected();
        assert_eq!(corrected, expected_corrected);
        assert_eq!(better, expected_better);
    }

    #[test]
    fn the_key_tail_waits_across_a_fragment_boundary() {
        // "\"corrected\"" split in the middle: the scanner must keep the prefix.
        let (corrected, better) =
            collect(&[b"{\"correc", b"ted\":\"fixed\",\"better\":\"nicer\"}"]);
        assert_eq!(corrected, "fixed");
        assert_eq!(better, "nicer");
    }

    #[test]
    fn a_unicode_escape_split_across_fragments_waits() {
        let (corrected, _) = collect(&[
            br#"{"corrected":"A \u0"#,
            br#"0e9"#,
            br#"h B","better":"x"}"#,
        ]);
        assert_eq!(corrected, "A éh B");
    }

    #[test]
    fn a_surrogate_pair_split_between_its_two_escapes_reconstructs_the_emoji() {
        let (corrected, _) =
            collect(&[br#"{"corrected":"Hi \uD83D"#, br#"\uDC4B!","better":"x"}"#]);
        assert_eq!(corrected, "Hi 👋!");
    }

    #[test]
    fn fields_may_arrive_in_either_order() {
        let mut deltas = Vec::new();
        let mut stream = ImprovementStream::new();
        stream.feed(
            br#"{"better":"second","corrected":"first"}"#,
            &mut |delta| deltas.push(delta.clone()),
        );

        assert_eq!(
            deltas,
            vec![
                ImproveDelta {
                    part: ImprovePart::Better,
                    text: "second".to_owned(),
                },
                ImproveDelta {
                    part: ImprovePart::Corrected,
                    text: "first".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_key_named_inside_another_value_does_not_hijack_the_stream() {
        let (corrected, better) =
            collect(&[br#"{"corrected":"say \"better\": no","better":"fine"}"#]);
        assert_eq!(corrected, "say \"better\": no");
        assert_eq!(better, "fine");
    }

    #[test]
    fn nothing_is_reported_before_a_field_value_starts() {
        let mut deltas = Vec::new();
        ImprovementStream::new().feed(br#"{"corrected":"#, &mut |delta| deltas.push(delta.clone()));
        assert!(deltas.is_empty());
    }
}
