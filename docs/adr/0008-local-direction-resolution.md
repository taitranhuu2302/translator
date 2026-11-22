# Direction is resolved locally, before the request

Every translation the app runs has a known Direction before any provider is asked. `LanguagePair::direction_for` reads the text with a local heuristic — Vietnamese letters or tone marks mean Vietnamese, any other Latin letter means English, no recognisable letter is no guess — chooses the Target or its partner from that, and sends both languages with the request. Providers are never asked to choose the direction.

The provider's own reported detection is still carried back (`Translation::detected_source`) as a fact about the answer, but it never decides where text is translated. That matters because the provider ladder falls back: MyMemory takes a `langpair` and nothing else, so a provider-chosen direction would work on Azure and then silently stop working when the keyless fallback takes over. Deciding locally also makes the result predictable before the first byte leaves the Mac: the same text always goes the same way, and a Swap is a reversal of the direction the answer was actually produced in.

The heuristic is deliberately shallow. Unaccented Vietnamese (`toi di hoc`) is indistinguishable from English to it and is treated as English. Text in a non-Latin script (Japanese, Chinese, Cyrillic) carries no Latin letter, so it is sent from the pair's partner to its Target rather than detected.

**Considered options:** ask Azure to detect and translate in one request; run a language-identification crate such as `whatlang`; build an unaccented-Vietnamese word list. The first ties the two-language behaviour to one provider in the ladder. The second is trigram-based and unreliable on the short selections this app mostly sees, at the cost of 70 languages of model data. The third was rejected for v1 because it misreads ordinary English more often than it rescues unaccented Vietnamese.

**Consequences:** the Direction is a value every surface can show, so the Translate Window and the popup name it instead of leaving the user to infer it. `Translation` carries the language it ran from, and Swap reverses that value rather than guessing again. A future third language must be added to the heuristic and to `OFFERED_LANGUAGES` together.

This supersedes the sentence in `.scratch/translator-v1/spec.md` that said detection uses the provider's reported language when available.
