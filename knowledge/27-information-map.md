# 27 — The information map: what a phone can give Verdure, and the shape it lands in

**Status:** 🟡 Provisional (written 2026-10-22 from the founder's direction — "map out all the types of information we can get from the user's phone, mark things by events, know that this person is associated with this event at this time" — before the Pixel-first build of `25`'s feeds. The platform-availability facts are stated from the platform references; the two marked ⚠ are unverified.)

*Role: the **inventory and the shape**. `25` decides how Verdure builds context about the user and how a question is answered; this file lists every source of information a phone can offer, says where each one is reachable, and derives from that list the four nouns and one link that the store (`app/src-tauri/src/context.rs`) is built around. Read `25` first, then this; change the shape here before changing the code.*

## Why a map before a feed

The first version of the store (2026-10-21) was shaped by its first source: a message table — thread, sender, direction, text. That is the wrong foundation for a phone, because most of what a phone knows is not a message. A calendar entry, a photo's time and place, a call, a visit to a place — each is a thing that happened at a time, often with people in it. The questions that matter most ("whose birthday is coming up", "when did I last see Tunde", "what happened at the wedding") cut across sources, so the store's first axis has to be **time**, its second **people**, and the thing that ties a person to a time is an **event**. The founder put it in one line: *mark things by events, and know that this person is associated with this event at this time.*

Reshaping now is free — no user has data in this store, no build has shipped it. Reshaping after the Pixel listener has filled it would mean a migration on a phone.

## The map

Each source is listed with what it contributes (the nouns below), where it is reachable, what it costs the user in permission and trust, and when Verdure takes it. **Pixel first** (founder, 2026-10-22): the Android column is the one being built; iOS is stated so that no iOS feature is ever planned on an Android-only source.

| Source | Contributes | Android | iOS / macOS | Permission and trust | When |
|---|---|---|---|---|---|
| **Messages** | people, time, text, threads | **Live**, through the notification listener (`[OQ-100]`): `MessagingStyle` sender/text/time/group/title. Not on Android Go-class phones on Android ≤ 10 (`[OQ-102]`); OTPs redacted on 15 | Never live. Only what the user shares in (a chat export) | Notification access: a system settings toggle with Play's prominent-disclosure requirement; the most powerful permission Verdure asks for | **Now** — the product |
| **Chat exports** | the past of one conversation | Share sheet → Verdure | Share Extension → Verdure | None beyond the share | Built (`25`); UI later |
| **Calendar** | **events** (title, time span, place, attendees), the *given* events | `CalendarContract.Instances` — recurrence expanded by the OS | `EventKit` (iOS 17+ full access) | `READ_CALENDAR`; a normal in-app prompt | **Next after messages** — the event source |
| **Contacts** | people, their keys (numbers, emails), birthdays, relationships | `ContactsContract` | `Contacts` framework | `READ_CONTACTS`; in-app prompt | With calendar — this is what joins "Tunde on WhatsApp" to "Tunde in Contacts" and answers the birthday question without a model |
| **Photos, metadata only** | time, place; *inferred* events (a cluster at one place on one evening) | `MediaStore` with `READ_MEDIA_IMAGES` — Play treats it as sensitive and steers apps to the photo picker ⚠ (whether a metadata-only scan passes review is unverified) | `PhotoKit` for time and location; the names in Photos' People albums are **not** exposed to apps ⚠ (unverified beyond the public API surface) | High: "all your photos" is the prompt the user sees, even if only times and places are read | Later; opt-in |
| **Location** | places, visits (time at a place) | Fused location with background access | `CLLocation` significant-change / visits | The most sensitive permission on the phone; background access is a separate, scary prompt | Late, opt-in, and only if visits earn their keep |
| **Calls** | people, time (who called, how long) | `CallLog` — Play grants it only to the default Phone/Assistant handler | No API | Play-gated | Not in v1; the *notification* of a missed call still arrives through the listener |
| **SMS** | messages | `READ_SMS` — default SMS handler only | No API | Play-gated | Not in v1; SMS notifications arrive through the listener |
| **Reminders / tasks** | things to do, with a time | No standard provider | `EventKit` reminders | Prompt | Later |
| **Email** | people, time, text | IMAP / Gmail API, by connecting an account | Same | Account-level consent | Later; large |
| **Health** | numbers over time | Health Connect | HealthKit | Prompt | Out of this shape (no people, no events) |
| **Verdure's own chats and dictation** | what the user told Verdure | Have it | Have it | None | On disk since 2026-10-23 (`chat.jsonl`, `22`); not yet indexed into the store |
| **Any other notification** — reminders, receipts, deliveries, email subjects | time, text, app | **Live**, same listener, as a `notice` observation; every app unless unticked (2026-10-23) | Never | Same permission as messages | **Now** |
| Screen time, browsing history, clipboard | — | Partly reachable | Not reachable | Trust cost too high for the value | **Out** |

**What the table decides.** Five sources contribute people-and-time: messages, calendar, contacts, photos, location. They define the shape. Everything else either fits the same shape later (calls, reminders, email are observations with people and a time) or stays outside it (health).

## The shape: four nouns and one link

- **Person** — as in `25`: one row per human, the names they go by (`person_alias`), their keys in each app (`identity`). The user is a person with `is_self`.
- **Place** — a name, optional coordinates, optional address. "Tunde's house", "The Wheatbaker", a photo's GPS fix resolved to a name later.
- **Event** — a time span, a title, an optional place, and an **origin**: *given* (a calendar entry, with the app and the OS's id so it can be re-read) or *inferred* (a cluster of photos; a message thread's "see you Friday" that became a Friday). Given events are the truth of the calendar; the store keeps a thin row (title, span, place, OS id) so people can be linked to it, and re-reads the OS at question time.
- **Observation** — anything recorded with a time: a message, a call, a photo's metadata, a visit, a note. Every observation has a kind, a time, an optional place and an optional event; a message additionally has its thread, sender, sender's person and direction. **One table, one time axis**, so "what happened on the 25th" is one query across every source, and deleting a person or a thread is one path.
- **Participation** — *person was at event*, with a role (`organiser`, `invited`, `attended`, `mentioned`) and the source that said so (the calendar's attendee list, a photo, a message). This is the founder's sentence as a table.

Facts (`25`: birthday, plan, amount, deadline, address — bi-temporal, each pointing at the observation it came from) stay as they are. Keyword search covers observations with text; events are found by time and by person, and by their title.

**Why one observation table and not one table per kind** (design-it-twice, `skills/architecture.md`): a table per kind (messages, calls, photos…) keeps each row tidy but makes every cross-source question a UNION, every delete a fan-out, and every new source a schema change. One table with a kind column and a few nullable, kind-specific columns is deeper: the time index, the search index, the event link and the deletes are written once. The cost is nullable columns on a message row — accepted.

**What changed from `25`'s first design:** `items` (messages only) became `observation` (any kind); `place`, `event` and `participation` are new; `25`'s "calendar is never copied" is softened to "a calendar event gets a thin row with its OS id, and the OS stays the truth" — a participation link needs a stable row to point at. Contacts are still not copied: only the alias/identity link lives in the store.

## Pixel first

The Android build is where this becomes real: the notification listener (`[OQ-100]`) fills observations of kind *message*; calendar and contacts follow as the first event and people sources. Nothing in this file is planned for iOS until it works on the Pixel 8a. What iOS gets later is exactly what its column above allows: calendar, contacts, exports, and Verdure's own conversations.

## Open Questions

- `[OQ-114]` Photos as a source: whether a metadata-only `MediaStore` scan passes Play review under the photo-picker policy, and whether iOS exposes the identities behind Photos' People albums to third-party apps at all (the public `PhotoKit` surface suggests not). Both unverified; photos stay opt-in and later until answered.
- `[OQ-100]`, `[OQ-102]` as in `25`: the listener spike and the floor that cannot bind listeners.
