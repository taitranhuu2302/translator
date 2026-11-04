# 06: Azure F0 and the provider ladder

**What to build:** The user enters an Azure Translator key once, and from then on translations go through Azure with the keyless provider behind it. When Azure refuses — bad key, exhausted quota — the app still delivers a translation and says which provider answered. This is where the app stops being keyless-only and starts behaving like the provider ladder in the ADR.

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] A Settings window accepts an Azure key and stores it in the Keychain, never in user defaults or a file in the app bundle
- [ ] With a key configured, translation goes through Azure and the result records which provider produced it
- [ ] An auth or quota failure at Azure falls back to the keyless provider transparently, and the result still appears
- [ ] A provider that failed with an auth or quota error is not retried for the rest of the session
- [ ] With no key configured, the app uses the keyless provider without showing an error
- [ ] Errors name the provider and say what would fix it
