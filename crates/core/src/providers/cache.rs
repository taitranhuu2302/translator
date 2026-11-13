//! An in-memory, least-recently-used cache in front of the provider ladder.
//! Translating the same text in the same direction twice costs one request; the
//! same text in another direction is a different question and is asked again.
//! Failures are never cached: a network blip must not poison the next attempt.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::ProviderError;
use crate::language::Language;
use crate::model::{Improvement, ProviderId, ProviderTranslation};
use crate::ports::{
    ImproveRequest, TranslateRequest, TranslationProvider,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    text: String,
    source: Option<Language>,
    target: Language,
}

impl Key {
    fn of(request: &TranslateRequest) -> Self {
        Self {
            text: request.text.clone(),
            source: request.source.clone(),
            target: request.target.clone(),
        }
    }
}

struct Entry {
    answer: ProviderTranslation,
    used_at: u64,
}

struct State {
    entries: HashMap<Key, Entry>,
    clock: u64,
}

impl State {
    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    fn get(&mut self, key: &Key) -> Option<ProviderTranslation> {
        let used_at = self.tick();
        let entry = self.entries.get_mut(key)?;
        entry.used_at = used_at;
        Some(entry.answer.clone())
    }

    fn insert(&mut self, key: Key, answer: ProviderTranslation, capacity: usize) {
        if capacity == 0 {
            return;
        }
        if !self.entries.contains_key(&key) && self.entries.len() >= capacity {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used_at)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        let used_at = self.tick();
        self.entries.insert(key, Entry { answer, used_at });
    }
}

/// A provider wrapper with its own bounded cache. `capacity` is the number of
/// completed translations kept; zero disables caching.
pub struct CachedProvider {
    inner: Arc<dyn TranslationProvider>,
    capacity: usize,
    state: Mutex<State>,
}

impl CachedProvider {
    pub fn new(inner: Arc<dyn TranslationProvider>, capacity: usize) -> Self {
        Self {
            inner,
            capacity,
            state: Mutex::new(State {
                entries: HashMap::new(),
                clock: 0,
            }),
        }
    }
}

impl TranslationProvider for CachedProvider {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        let key = Key::of(request);
        if let Some(answer) = self.state.lock().expect("cache lock").get(&key) {
            return Ok(answer);
        }
        let answer = self.inner.translate(request)?;
        self.state
            .lock()
            .expect("cache lock")
            .insert(key, answer.clone(), self.capacity);
        Ok(answer)
    }

    /// Improve is left to the model provider; caching corrected prose would keep
    /// stale wording in front of the user for longer than it helps.
    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        self.inner.improve(request)
    }

    /// The cache sits in front of the ladder, so it must not hide which rungs the
    /// session has skipped.
    fn skipped(&self) -> Vec<ProviderId> {
        self.inner.skipped()
    }

    fn clear_skip(&self, provider: ProviderId) {
        self.inner.clear_skip(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProviderId;
    use crate::testing::FakeProvider;

    fn request(text: &str, target: Language) -> TranslateRequest {
        TranslateRequest {
            text: text.to_owned(),
            source: Some(Language::english()),
            target,
        }
    }

    fn translation(text: &str) -> ProviderTranslation {
        ProviderTranslation {
            text: text.to_owned(),
            provider: ProviderId::MyMemory,
            detected_source: None,
        }
    }

    fn cached(fake: &FakeProvider, capacity: usize) -> CachedProvider {
        CachedProvider::new(Arc::new(fake.clone()), capacity)
    }

    #[test]
    fn the_same_text_and_direction_is_answered_without_a_second_request() {
        let fake = FakeProvider::responding_with(translation("Xin chào"));
        let provider = cached(&fake, 8);

        let first = provider
            .translate(&request("Hello", Language::vietnamese()))
            .expect("the first request is answered");
        let second = provider
            .translate(&request("Hello", Language::vietnamese()))
            .expect("the second is a cache hit");

        assert_eq!(first, second);
        assert_eq!(fake.requests().len(), 1, "only one request reached the provider");
    }

    #[test]
    fn the_same_text_in_another_direction_is_a_different_question() {
        let fake = FakeProvider::responding_with(translation("anything"));
        let provider = cached(&fake, 8);

        provider
            .translate(&request("Hello", Language::vietnamese()))
            .expect("first direction");
        provider
            .translate(&request("Hello", Language::english()))
            .expect("second direction");

        assert_eq!(fake.requests().len(), 2);
    }

    #[test]
    fn a_failure_is_never_cached() {
        let fake = FakeProvider::default().failing_with(ProviderError::Offline);
        let provider = cached(&fake, 8);

        assert!(provider
            .translate(&request("Hello", Language::vietnamese()))
            .is_err());
        assert!(provider
            .translate(&request("Hello", Language::vietnamese()))
            .is_err());

        assert_eq!(
            fake.requests().len(),
            2,
            "the second attempt must reach the provider again"
        );
    }

    #[test]
    fn the_least_recently_used_translation_is_evicted() {
        let fake = FakeProvider::responding_with(translation("anything"));
        let provider = cached(&fake, 2);

        for text in ["one", "two"] {
            provider
                .translate(&request(text, Language::vietnamese()))
                .expect("answered");
        }
        // Touch "one" so "two" becomes the least recently used.
        provider
            .translate(&request("one", Language::vietnamese()))
            .expect("cache hit");
        provider
            .translate(&request("three", Language::vietnamese()))
            .expect("answered");

        assert_eq!(fake.requests().len(), 3);

        provider
            .translate(&request("one", Language::vietnamese()))
            .expect("still cached");
        assert_eq!(fake.requests().len(), 3, "\"one\" was not evicted");

        provider
            .translate(&request("two", Language::vietnamese()))
            .expect("asks again");
        assert_eq!(fake.requests().len(), 4, "\"two\" was evicted");
    }

    #[test]
    fn improve_is_not_cached() {
        let fake = FakeProvider::improving_with(Improvement {
            corrected: "hello".to_owned(),
            better: "Hello.".to_owned(),
        });
        let provider = cached(&fake, 8);
        let request = ImproveRequest {
            text: "helo".to_owned(),
        };

        provider.improve(&request).expect("first");
        provider.improve(&request).expect("second");

        assert_eq!(fake.improve_requests().len(), 2);
    }
}
