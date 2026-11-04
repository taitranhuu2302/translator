# Translation provider ladder: Azure F0, MyMemory, Groq

The original brief asked for Google's API. Google Cloud Translation v3 cannot authenticate with an API key (service account only, which cannot be shipped in a desktop app), and v2 requires billing to be enabled — 500k characters/month against Azure's 2M for the same key-only auth. The undocumented `client=gtx` endpoint is a terms-of-service violation and returned HTTP 429 when tested.

Translation of short text goes to Azure AI Translator F0: 2,000,000 characters/month, `Ocp-Apim-Subscription-Key` plus optional region, full Vietnamese support. With no Azure key configured, MyMemory serves as the keyless fallback. Improve, long sentences and explanations go to Groq `gpt-oss-120b` with low reasoning effort; Gemini is the last-resort AI fallback only, because its free tier permits using submitted content to improve Google's products.

**Consequences:** Azure requires an Azure subscription and a card at signup, and the free account must be converted to pay-as-you-go after 30 days or services are disabled (the F0 resource itself stays free). MyMemory allows 5,000 characters/day anonymously (50,000 only with a contact email in `de`) and caps requests at 500 **bytes**, so it needs a sentence-boundary chunker and a local cache. Provider order lives behind one protocol so the ladder can change without touching the UI.
