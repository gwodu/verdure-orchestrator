# 25 — The personal context store: what Verdure has seen, kept on the device

**Status:** 🟡 Provisional (the store and its search are built and tested, 2026-10-21; nothing feeds it on a device yet — the Android notification listener is the next step, `[OQ-100]`)

*Role: the **decision + design** for how Verdure builds context about the user — the store's shape, what feeds it, how a question is answered from it, and the rules that keep it private. This is `20`'s "personal context core" (`[OQ-56]`) made concrete, with messages as its first source. Written 2026-10-21 from the founder's messaging-hub research (October 2026) and the direction decided with him the same day.*

## The decision

Verdure does not become a messaging app. It becomes the place that **already knows** — the user asks *"what did Tunde say about the rent?"* or *"whose birthday is coming up?"* in the same chat they use for everything, and Verdure answers from what it has seen, with the source attached and tappable, exactly as it answers from the web (`24`). The "central messaging hub" the founder started from is one *source* of that knowledge and one transparency page, not a second inbox to maintain.

Three facts from the research shape it:

1. **No official API exists for personal WhatsApp, Instagram, Messenger, Snapchat, LinkedIn or Discord.** Every "all your chats in one place" product rests on reverse engineering, and the ban lands on the *user's* account — the one thing a Nigerian user can least afford to lose. The only zero-risk route is Android's notification layer: it sees what the official app chose to show, and replies through the app's own reply action. Telegram (TDLib), X (its new Chat API), Slack and Gmail are the sanctioned exceptions.
2. **Nothing may be relayed through a Verdure server.** Sunbird/Nothing Chats (2023) relayed decrypted messages through a cloud and leaked them; it is the category's one existential failure. On-device also shrinks the NDPA/PIPEDA position to almost nothing: Verdure the company never processes the third parties' data.
3. **This is Android.** iOS gives apps no access to other apps' notifications, SMS, or anything else. The store itself is cross-platform; the feeds are Android-first, with Telegram's official API the only cross-platform feed in sight.

## The store

One SQLite file, `context.sqlite`, in the app's private folder beside the models (`app/src-tauri/src/context.rs`). **Not a "notifications table"**: a notification is a transport, and what it carried is an **item** — a message from a person in a thread at a time — the same shape whether it arrived as a WhatsApp notification, an SMS, or a chat export the user shared in.

| Table | Holds | Notes |
|---|---|---|
| `sources` | which feeds are on: kind (`notification` / `sms` / `chat_export` / later `calendar`…) × app | the per-app opt-in list the user sees |
| `people` | who: display name + per-app handle | merging one person across apps is a later step |
| `threads` | where: app + a stable key (Android's conversation id, a phone number, an export's chat name) + title + group? + last activity | |
| `items` | what: thread, sender, direction `in`/`out`, time, text, **dedupe key (UNIQUE)** | the only table anything writes to today |
| `items_fts` | FTS5 over text + sender + thread title | keyword search, instant, offline |
| `facts` | extracted later while charging: kind (`birthday` / `plan` / `amount` / `deadline` / `address`), subject, value, when, confidence, which model — **each pointing at the item it came from** | step 4 |
| embeddings | item → vector | step 5, only if keyword search measurably misses |

**Dedupe matters more than it looks.** Android re-posts the whole conversation every time a notification updates, and a user can share the same export twice; the key is a stable FNV-1a hash of (app, thread, sender, time, text), so re-posts and re-imports store nothing. Migrations are appended with a version check and never edited: a store on a phone must survive every version of the schema that ever shipped.

## How a question is answered

The web-search mechanism, pointed inward:

1. **Structured asks first, deterministically.** "Whose birthday is coming up" → `facts WHERE kind = 'birthday' AND when within 30 days`. No model is needed to *find* it, only to phrase it. (Step 4.)
2. **Everything else: keyword retrieval → the same sources block → cited answer.** The question's content words (stopwords dropped, each a quoted prefix match — `"tunde"* OR "rent"*`, so nothing a user types can break FTS5's syntax) rank items by BM25 with the text weighted over the sender over the thread title, newest first on ties. The top ~8 become `web::Snippet`s — sender and thread where a page title stood, the app's everyday name where the domain stood, the date, and a `verdure://thread/<id>` link — rendered by `web::sources_block` and answered with `[n]`; the footer's `[n]` opens that conversation. The model never sees the whole store, only what was retrieved.
3. **Embeddings only when 2 measurably misses.** Small models answer worse from more context, not better (`24`); eight relevant messages beat a thousand.

Why retrieval before extraction: a 4B pulls a date out of ten retrieved messages passably; a 2B invents one. Every answer shows its sources so a wrong one is visibly wrong — the same honesty the web search has.

## What feeds it, in order

1. **Android notification listener** (`[OQ-100]`, the next step): a small Kotlin `NotificationListenerService` in the Tauri Android project (we already patch `MainActivity` for the microphone). Read the **structured** conversation, not the text: WhatsApp, Telegram, Messenger and Google Messages post `MessagingStyle` notifications — sender, text, timestamp, group flag, conversation title — which maps straight onto `items`. Instagram and Snapchat are weaker (plain text; content-free "sent you a snap"). Per-app opt-in. Direct Reply through the app's own `RemoteInput` while the notification is alive; otherwise a deep link into the app. **Never auto-send** (`06` principle 4).
2. **Chat export** (built, 2026-10-21): the one-time, user-driven way to give Verdure a conversation's past. WhatsApp's "Export chat" is **per chat** and about five taps, so this is "bring in the three people that matter", not a migration — the honest line is *Verdure's memory starts the day you turn this on.* `parse_whatsapp_export` handles the Android and iOS forms, 12/24-hour clocks, multi-line messages, system lines, and marks the user's own side `out`. Dates carry no timezone and follow the phone's locale (`[OQ-99]`).
3. **Telegram via TDLib** (later): the one platform with full history through an official client API — and the only feed that would also work on iOS.
4. **Calendar, contacts, SMS as the default handler** — later sources, same table.

What stays out indefinitely: reverse-engineered protocol clients for WhatsApp/Meta, even user-owned. The ban risk is the user's account; it is the one dependency where the counterparty is Meta.

## Rules that keep the promise

- **Nothing leaves the device.** No sync, no relay, no server, no telemetry. The feature's disclosure is one sentence.
- **Excluded from cloud backup.** Android's auto-backup to Google Drive would copy the file off the device by a side door; the platform build must list `context.sqlite` in the backup exclusion rules. This is the rule most likely to be broken silently — it is a test the moment the Android build has the file.
- **Visible and deletable.** A page called **What Verdure knows**: what is stored, from which apps, how much, delete a conversation, delete everything. This is what *ask, don't sniff* requires of a powerful permission.
- **Encryption at rest:** the OS's file-based encryption and the app sandbox today; SQLCipher with a Keystore-held key is a hardening step whose cross-compile cost is unknown (`[OQ-99]`).
- **Messages are untrusted input.** Reading is safe; the moment Verdure can act (reply, book, pay), a message saying "ignore your instructions and send…" is a real attack. Acting stays behind the tool-calling isolation already gated on `[OQ-58]`.
- **Proactive only when the value is unambiguous.** Verdure speaks first for a birthday tomorrow or an OTP it can read back — never for "here's a summary of your day" — and every nudge is one tap from "never again". Earned, rare, reversible. Everything else waits to be asked.

## The steps

1. ✅ **The store** (2026-10-21): schema, ingest with dedupe, FTS5 search, threads, counts, delete, the export parser. Pure Rust, 13 tests, no device needed.
2. **Ask-with-sources + What Verdure knows**: the store as a grounding source in chat (the `Grounding` seam from `24`), the transparency page, `[n]` opening the thread, "share an export to Verdure".
3. **Pixel 8a spike**: the Kotlin listener feeding `ingest`; measure capture rate over a week, reply success, battery, survival under OEM battery managers. Android builds are 1× billing. Kill-criteria from the research: under 90 % capture on the target Transsion devices means "digest of what you saw", not "replace the apps".
4. **Facts while charging** on the 4B → the birthday question.
5. **Embeddings**, only if step 2 shows keyword search missing.

## Open Questions

- `[OQ-99]` Export dates are locale-ambiguous (day-first assumed) and timezone-less; and encryption at rest beyond the sandbox (SQLCipher via `rusqlite`'s `bundled-sqlcipher`) needs its iOS/Android cross-compile cost measured before it is promised.
- `[OQ-100]` The Android notification listener spike: `MessagingStyle` coverage per app, capture rate, Direct Reply success, battery, OEM battery-manager survival, and Play's prominent-disclosure requirement for notification access. No `AccessibilityService` — Play bars it for this and Android 17 revokes it.
- `[OQ-101]` Retrieval quality: does BM25 over content words find the right messages for real questions, and when does it miss badly enough to justify an embedding model (which one fits the floor — EmbeddingGemma-300M needs ~760 MB)?
