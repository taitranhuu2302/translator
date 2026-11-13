//! Splitting long text so a provider can translate it in pieces.
//!
//! MyMemory's per-request cap is 500 **Unicode scalar values**, not bytes (see
//! `translator-notes/v1/mymemory.md`), so every count here is `chars()`. Chunks
//! carry their trailing whitespace, which makes `join_chunks` an exact
//! concatenation: nothing is trimmed, reordered or lost in the round trip.

/// The most characters MyMemory accepts in one `q`.
pub const MAX_CHUNK_CHARS: usize = 500;

/// Split `text` into pieces of at most `limit` characters. A split prefers the
/// end of a sentence, then any whitespace, and only cuts mid-word when a single
/// sentence is longer than the budget. The pieces concatenate back to `text`.
pub fn chunk_text(text: &str, limit: usize) -> Vec<String> {
    assert!(limit > 0, "a chunk must be able to hold one character");
    let mut chunks = Vec::new();
    let mut start = 0;
    while text[start..].chars().count() > limit {
        let split = split_point(&text[start..], limit);
        chunks.push(text[start..start + split].to_owned());
        start += split;
    }
    chunks.push(text[start..].to_owned());
    chunks
}

/// Rejoin translated chunks in order. Whitespace lives at the end of the chunk
/// it followed, so this is a plain concatenation.
pub fn join_chunks(chunks: &[String]) -> String {
    chunks.concat()
}

/// Where to cut a slice that is known to hold more than `limit` characters.
/// Returns a byte offset that is a character boundary.
fn split_point(text: &str, limit: usize) -> usize {
    let mut end_of_window = text.len();
    let mut last_sentence_end = None;
    let mut last_space_end = None;
    let mut before_whitespace = None;

    for (seen, (byte, character)) in text.char_indices().enumerate() {
        if seen == limit {
            end_of_window = byte;
            break;
        }
        let after = byte + character.len_utf8();
        if character.is_whitespace() {
            let candidate = if before_whitespace.is_some_and(is_sentence_end) {
                &mut last_sentence_end
            } else {
                &mut last_space_end
            };
            *candidate = Some(after);
        } else {
            before_whitespace = Some(character);
        }
    }

    last_sentence_end
        .or(last_space_end)
        .unwrap_or(end_of_window)
        .min(end_of_window)
        .max(1)
}

/// Sentence punctuation, including the closing marks that usually follow it.
fn is_sentence_end(character: char) -> bool {
    matches!(
        character,
        '.' | '!' | '?' | '…' | '。' | '！' | '？' | '؛' | '।' | '"' | '\'' | '”' | '’' | ')' | ']'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split_and_rejoin(text: &str, limit: usize) -> Vec<String> {
        let chunks = chunk_text(text, limit);
        assert_eq!(
            join_chunks(&chunks),
            text,
            "the chunks must concatenate back to the original text"
        );
        for chunk in &chunks {
            assert!(
                chunk.chars().count() <= limit,
                "chunk of {} chars exceeds the {limit}-char budget: {chunk:?}",
                chunk.chars().count()
            );
        }
        chunks
    }

    #[test]
    fn short_text_is_one_unchanged_chunk() {
        assert_eq!(chunk_text("Xin chào", 500), vec!["Xin chào"]);
        assert_eq!(chunk_text("", 500), vec![""]);
    }

    #[test]
    fn a_five_hundred_character_vietnamese_text_stays_one_chunk() {
        // 500 characters with diacritics is well over 500 bytes; the budget is
        // characters, and this is the boundary the provider verified.
        let text = "ế".repeat(500);
        assert_eq!(text.len(), 1_500, "the fixture must be multi-byte");
        assert_eq!(chunk_text(&text, 500), vec![text]);
    }

    #[test]
    fn a_long_text_is_split_and_rejoins_losslessly_with_diacritics_intact() {
        let sentence = "Đây là một câu tiếng Việt có dấu. ";
        let text = sentence.repeat(40);
        let chunks = split_and_rejoin(&text, 500);
        assert!(chunks.len() > 1, "the fixture must need several chunks");
        // Every chunk ends at a sentence, so no Vietnamese word was cut in half.
        for chunk in &chunks[..chunks.len() - 1] {
            assert!(
                chunk.trim_end().ends_with('.'),
                "a chunk should end at a sentence: {chunk:?}"
            );
        }
    }

    #[test]
    fn a_sentence_longer_than_the_budget_is_cut_at_the_budget() {
        let text = "a".repeat(1_200);
        let chunks = split_and_rejoin(&text, 500);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chars().count(), 500);
        assert_eq!(chunks[1].chars().count(), 500);
        assert_eq!(chunks[2].chars().count(), 200);
    }

    #[test]
    fn whitespace_and_newlines_are_preserved_exactly() {
        let text = format!("{}\n\n{}", "a".repeat(480), "b".repeat(60));
        let chunks = split_and_rejoin(&text, 500);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].ends_with("\n\n"));
    }

    #[test]
    fn a_split_never_lands_inside_a_scalar_value() {
        // Astral characters are four bytes each. A byte-based split would panic
        // or corrupt them; a character-count split cannot.
        let text = "😀".repeat(1_001);
        let chunks = split_and_rejoin(&text, 500);
        assert_eq!(chunks.len(), 3);
    }
}
