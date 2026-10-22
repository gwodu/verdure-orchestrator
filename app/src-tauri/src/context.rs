//! The personal context store (knowledge/25, shaped by knowledge/27): what
//! Verdure has seen, kept on the device so it can answer "what did Tunde say
//! about the rent?" or "when did I last see Ada?" the way it answers from the
//! web — from retrieved sources, cited, tappable.
//!
//! One SQLite file in the app's private folder. **Four nouns and one link**
//! (knowledge/27): a *person*, a *place*, an *event* (a time span with a
//! title, a place and an origin), an *observation* (anything recorded at a
//! time — a message, a call, a photo's metadata, a visit, a note), and
//! *participation* (this person was at this event, in this role). A message is
//! one kind of observation; it also carries its thread, sender and direction.
//! One observation table means one time axis — "what happened on the 25th"
//! is one query across every source — and one delete path.
//!
//! The store is **entity-centric and structured first** (knowledge/25): people
//! are rows, every observation points at the person it came from, and the
//! questions that matter most — "the last thing Tunde said", "events with
//! Ada" — are indexed lookups, not retrieval. Keyword search (FTS5) is the
//! second tier; vectors are a later, optional third. Small models ignore or
//! are misled by retrieved text far more than they are helped by it (Pandey
//! et al. 2026, arXiv 2603.11513), so the fewer and more exact the records,
//! the better.
//!
//! Nothing here leaves the device. There is no sync, no relay, no server. The
//! file is excluded from cloud backup by the platform build (knowledge/25),
//! and the user can see and delete all of it from "What Verdure knows".
#![allow(dead_code)]

use std::path::Path;

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::web::Snippet;

/// Schema version. Version 1 was redefined on 2026-10-22 (knowledge/27) before
/// any build shipped the file; from the first shipped build on, migrations are
/// appended with a version check and never edited in place — a store on a
/// phone has to survive every version of this schema that ever shipped.
const SCHEMA_VERSION: i64 = 1;

/// Where an observation came from. Strings, not an enum, because new sources
/// are added by the platform code that produces them, not by this module.
pub(crate) mod source {
    pub const NOTIFICATION: &str = "notification";
    pub const SMS: &str = "sms";
    pub const CHAT_EXPORT: &str = "chat_export";
    pub const CALENDAR: &str = "calendar";
    pub const PHOTOS: &str = "photos";
    pub const CALL_LOG: &str = "call_log";
    pub const USER: &str = "user";
}

/// What kind of thing an observation is (knowledge/27's map).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Message,
    Call,
    Photo,
    Visit,
    Note,
}

impl Kind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Kind::Message => "message",
            Kind::Call => "call",
            Kind::Photo => "photo",
            Kind::Visit => "visit",
            Kind::Note => "note",
        }
    }
    fn parse(s: &str) -> Kind {
        match s {
            "call" => Kind::Call,
            "photo" => Kind::Photo,
            "visit" => Kind::Visit,
            "note" => Kind::Note,
            _ => Kind::Message,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Someone wrote to the user.
    In,
    /// The user wrote (a reply sent through Verdure, or their side of an export).
    Out,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
        }
    }
    fn parse(s: &str) -> Direction {
        if s == "out" {
            Direction::Out
        } else {
            Direction::In
        }
    }
}

/// Where an event came from: the calendar said so, or Verdure worked it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    Given,
    Inferred,
}

impl Origin {
    fn as_str(self) -> &'static str {
        match self {
            Origin::Given => "given",
            Origin::Inferred => "inferred",
        }
    }
}

/// How a person relates to an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Organiser,
    Invited,
    Attended,
    Mentioned,
}

impl Role {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Role::Organiser => "organiser",
            Role::Invited => "invited",
            Role::Attended => "attended",
            Role::Mentioned => "mentioned",
        }
    }
}

/// One message as a source hands it in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewMessage {
    pub source: String,
    /// The app's package (`com.whatsapp`) or a well-known name (`sms`).
    pub app: String,
    /// Stable per conversation within the app: Android's conversation
    /// shortcut id, a phone number, or the export's chat name.
    pub thread_key: String,
    pub thread_title: String,
    pub is_group: bool,
    /// Display name of the sender; empty for the user's own messages.
    pub sender: String,
    /// The app's stable key for the sender when it has one (a MessagingStyle
    /// Person key or URI, a phone number); the name itself otherwise. This is
    /// what lets "Tunde" on WhatsApp and "Tunde Bakare" in SMS become one
    /// person once the user links them — or stay two until then.
    pub sender_key: String,
    pub direction: Direction,
    /// Unix milliseconds.
    pub at_ms: i64,
    pub text: String,
}

/// Any other observation — a call, a photo's metadata, a visit, a note — as a
/// source hands it in. Messages go through `NewMessage` because they carry a
/// thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewObservation {
    pub kind: Kind,
    pub source: String,
    pub app: String,
    /// The source's own id for this record (a MediaStore id, a call-log row
    /// id); part of the dedupe key so a rescan stores nothing twice.
    pub source_key: String,
    pub at_ms: i64,
    /// What there is to say about it ("Missed call", a note's words); may be
    /// empty for a photo.
    pub text: String,
    /// The person it came from or is about, if known.
    pub person_id: Option<i64>,
    /// A place name, resolved to a `place` row on the way in.
    pub place: Option<String>,
    pub event_id: Option<i64>,
}

/// A calendar entry or an inferred event, as a source hands it in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewEvent {
    pub title: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub place: Option<String>,
    pub origin: Origin,
    /// Which calendar app (for a given event); empty for an inferred one.
    pub app: String,
    /// The OS's id for the entry, so it can be re-read; empty for an inferred
    /// event, which then dedupes on its title and start.
    pub source_key: String,
}

/// What `ingest`/`record` did. Notifications re-post the whole conversation
/// every time they update, so most calls are duplicates — normal, not an error.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ingest {
    Inserted(i64),
    Duplicate,
}

/// A stored observation, as the UI and the prompt see it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Observation {
    pub id: i64,
    pub kind: &'static str,
    pub app: String,
    pub at_ms: i64,
    pub text: String,
    /// Messages only.
    pub thread_id: Option<i64>,
    pub thread_title: String,
    pub sender: String,
    pub sender_person_id: Option<i64>,
    pub direction: &'static str,
    pub place: Option<String>,
    pub event_id: Option<i64>,
}

/// An event, as the UI and the prompt see it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Event {
    pub id: i64,
    pub title: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub place: Option<String>,
    pub origin: &'static str,
    pub app: String,
}

/// One conversation, for the "What Verdure knows" list.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadSummary {
    pub id: i64,
    pub app: String,
    pub title: String,
    pub is_group: bool,
    pub last_at_ms: i64,
    pub items: i64,
    pub last_text: String,
}

/// Someone Verdure has heard from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Person {
    pub id: i64,
    pub display_name: String,
    /// How many observations came from them.
    pub items: i64,
}

/// A person at an event.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Participant {
    pub person: Person,
    pub role: String,
}

/// How much Verdure holds per app — the transparency number.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppCount {
    pub app: String,
    pub items: i64,
    pub threads: i64,
}

/// The answer to "who is X?" — the question every person-scoped lookup asks
/// first. Two Jacksons must become a question back to the user, never a
/// confident answer from the wrong one (knowledge/25).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PersonMatch {
    None,
    One(Person),
    Ambiguous(Vec<Person>),
}

pub(crate) struct Store {
    conn: Connection,
}

const OBSERVATION_COLUMNS: &str = "o.id, o.kind, o.app, o.at_ms, o.text, o.thread_id, COALESCE(t.title, ''), o.sender,
     o.sender_person_id, o.direction, p.name, o.event_id";

const OBSERVATION_FROM: &str = "FROM observation o LEFT JOIN threads t ON t.id = o.thread_id LEFT JOIN place p ON p.id = o.place_id";

impl Store {
    /// Open (creating if needed) the store at `path`.
    pub(crate) fn open(path: &Path) -> Result<Store, String> {
        let conn = Connection::open(path).map_err(|e| format!("couldn't open Verdure's memory: {e}"))?;
        Store::init(conn)
    }

    /// A throwaway store, for tests.
    pub(crate) fn open_in_memory() -> Result<Store, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Store::init(conn)
    }

    fn init(conn: Connection) -> Result<Store, String> {
        // WAL + NORMAL is the configuration that mattered most on Android
        // storage in the UT Austin SQLite benchmarking study (APSys 2017: up to
        // 11.8× over DELETE journaling); the page cache is capped at ~8 MB so
        // the store never competes with a model for RAM (knowledge/25).
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA cache_size = -8000; PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| e.to_string())?;
        let store = Store { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Idempotent: creates what is missing. New migrations are appended below
    /// with a version check, never edited in place.
    fn migrate(&self) -> Result<(), String> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version < 1 {
            self.conn
                .execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS sources (
                        kind TEXT NOT NULL,
                        app TEXT NOT NULL,
                        enabled INTEGER NOT NULL DEFAULT 1,
                        added_at_ms INTEGER NOT NULL,
                        PRIMARY KEY (kind, app)
                    );
                    -- People: a person, the names they go by, and the per-app
                    -- keys that are them (knowledge/25).
                    CREATE TABLE IF NOT EXISTS person (
                        id INTEGER PRIMARY KEY,
                        display_name TEXT NOT NULL,
                        is_self INTEGER NOT NULL DEFAULT 0
                    );
                    CREATE TABLE IF NOT EXISTS person_alias (
                        alias_norm TEXT NOT NULL,
                        person_id INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
                        kind TEXT NOT NULL,
                        PRIMARY KEY (alias_norm, person_id)
                    ) WITHOUT ROWID;
                    CREATE TABLE IF NOT EXISTS identity (
                        app TEXT NOT NULL,
                        source_key TEXT NOT NULL,
                        person_id INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
                        PRIMARY KEY (app, source_key)
                    ) WITHOUT ROWID;
                    -- Places: named once, reused by events and observations.
                    CREATE TABLE IF NOT EXISTS place (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        name_norm TEXT NOT NULL UNIQUE,
                        lat REAL,
                        lon REAL,
                        address TEXT
                    );
                    -- Events: a span with a title, a place and an origin
                    -- (given by a calendar, or inferred). A given event keeps
                    -- the OS's id so the OS stays the truth (knowledge/27).
                    CREATE TABLE IF NOT EXISTS event (
                        id INTEGER PRIMARY KEY,
                        title TEXT NOT NULL,
                        start_ms INTEGER NOT NULL,
                        end_ms INTEGER NOT NULL,
                        place_id INTEGER REFERENCES place(id) ON DELETE SET NULL,
                        origin TEXT NOT NULL,
                        app TEXT NOT NULL,
                        source_key TEXT NOT NULL,
                        dedupe_key TEXT NOT NULL UNIQUE
                    );
                    CREATE INDEX IF NOT EXISTS event_start ON event(start_ms);
                    -- The link: this person was at this event, in this role.
                    CREATE TABLE IF NOT EXISTS participation (
                        event_id INTEGER NOT NULL REFERENCES event(id) ON DELETE CASCADE,
                        person_id INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
                        role TEXT NOT NULL,
                        source TEXT NOT NULL,
                        PRIMARY KEY (event_id, person_id)
                    ) WITHOUT ROWID;
                    CREATE INDEX IF NOT EXISTS participation_person ON participation(person_id);
                    -- Threads: where a message lives.
                    CREATE TABLE IF NOT EXISTS threads (
                        id INTEGER PRIMARY KEY,
                        app TEXT NOT NULL,
                        thread_key TEXT NOT NULL,
                        title TEXT NOT NULL,
                        is_group INTEGER NOT NULL DEFAULT 0,
                        last_at_ms INTEGER NOT NULL DEFAULT 0,
                        UNIQUE (app, thread_key)
                    );
                    -- Observations: one table, one time axis, every kind.
                    -- The message-only columns are null for the other kinds.
                    CREATE TABLE IF NOT EXISTS observation (
                        id INTEGER PRIMARY KEY,
                        kind TEXT NOT NULL,
                        source TEXT NOT NULL,
                        app TEXT NOT NULL,
                        at_ms INTEGER NOT NULL,
                        text TEXT NOT NULL,
                        place_id INTEGER REFERENCES place(id) ON DELETE SET NULL,
                        event_id INTEGER REFERENCES event(id) ON DELETE SET NULL,
                        thread_id INTEGER REFERENCES threads(id) ON DELETE CASCADE,
                        sender TEXT NOT NULL DEFAULT '',
                        sender_person_id INTEGER REFERENCES person(id) ON DELETE SET NULL,
                        direction TEXT NOT NULL DEFAULT 'in',
                        dedupe_key TEXT NOT NULL UNIQUE
                    );
                    CREATE INDEX IF NOT EXISTS observation_at ON observation(at_ms);
                    CREATE INDEX IF NOT EXISTS observation_kind_at ON observation(kind, at_ms);
                    CREATE INDEX IF NOT EXISTS observation_thread_at ON observation(thread_id, at_ms);
                    CREATE INDEX IF NOT EXISTS observation_person_at ON observation(sender_person_id, at_ms);
                    CREATE INDEX IF NOT EXISTS observation_event ON observation(event_id);
                    -- Keyword tier. The index keeps its own copy of the text:
                    -- a contentless FTS5 table can't delete by rowid, and
                    -- records are small. `who` is the sender or the person;
                    -- `where_` is the thread title, the place, or the event.
                    CREATE VIRTUAL TABLE IF NOT EXISTS observation_fts USING fts5(
                        text, who, where_,
                        tokenize='unicode61 remove_diacritics 2'
                    );
                    -- Extracted later, while charging; every fact points at
                    -- its observation, and facts are invalidated, not deleted.
                    CREATE TABLE IF NOT EXISTS facts (
                        id INTEGER PRIMARY KEY,
                        observation_id INTEGER NOT NULL REFERENCES observation(id) ON DELETE CASCADE,
                        subject_person_id INTEGER REFERENCES person(id) ON DELETE SET NULL,
                        kind TEXT NOT NULL,
                        value TEXT NOT NULL,
                        when_ms INTEGER,
                        valid_from_ms INTEGER NOT NULL,
                        valid_to_ms INTEGER,
                        confidence REAL NOT NULL,
                        extracted_by TEXT NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS facts_kind_when ON facts(kind, when_ms);
                    PRAGMA user_version = 1;
                    ",
                )
                .map_err(|e| format!("couldn't prepare Verdure's memory: {e}"))?;
        }
        debug_assert!(SCHEMA_VERSION >= 1);
        Ok(())
    }

    // ----- writing -----------------------------------------------------------

    /// Store one message, or recognise it as already stored. The thread and
    /// the sender's person row are created on first sight.
    pub(crate) fn ingest(&mut self, m: &NewMessage) -> Result<Ingest, String> {
        let key = dedupe_key(&m.app, &m.thread_key, &m.sender_key, m.at_ms, &m.text);
        if self.has_observation(&key)? {
            return Ok(Ingest::Duplicate);
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO threads (app, thread_key, title, is_group, last_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(app, thread_key) DO UPDATE SET
               title = CASE WHEN excluded.title != '' THEN excluded.title ELSE title END,
               is_group = MAX(is_group, excluded.is_group),
               last_at_ms = MAX(last_at_ms, excluded.last_at_ms)",
            params![m.app, m.thread_key, m.thread_title, m.is_group as i64, m.at_ms],
        )
        .map_err(|e| e.to_string())?;
        let thread_id: i64 = tx
            .query_row(
                "SELECT id FROM threads WHERE app = ?1 AND thread_key = ?2",
                params![m.app, m.thread_key],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let person_id = match m.direction {
            Direction::Out => None,
            Direction::In => Some(resolve_person(&tx, &m.app, &m.sender_key, &m.sender)?),
        };
        tx.execute(
            "INSERT INTO observation (kind, source, app, at_ms, text, thread_id, sender, sender_person_id, direction, dedupe_key)
             VALUES ('message', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![m.source, m.app, m.at_ms, m.text, thread_id, m.sender, person_id, m.direction.as_str(), key],
        )
        .map_err(|e| e.to_string())?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO observation_fts (rowid, text, who, where_) VALUES (?1, ?2, ?3, ?4)",
            params![id, m.text, m.sender, m.thread_title],
        )
        .map_err(|e| e.to_string())?;
        note_source(&tx, &m.source, &m.app, m.at_ms)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Ingest::Inserted(id))
    }

    /// Store any other kind of observation (a call, a photo, a visit, a
    /// note), or recognise it as already stored.
    pub(crate) fn record(&mut self, o: &NewObservation) -> Result<Ingest, String> {
        let key = dedupe_key(&o.app, o.kind.as_str(), &o.source_key, o.at_ms, &o.text);
        if self.has_observation(&key)? {
            return Ok(Ingest::Duplicate);
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let place_id = match &o.place {
            Some(name) => Some(place_id(&tx, name)?),
            None => None,
        };
        let who: String = match o.person_id {
            Some(p) => tx
                .query_row("SELECT display_name FROM person WHERE id = ?1", params![p], |r| r.get(0))
                .optional()
                .map_err(|e| e.to_string())?
                .unwrap_or_default(),
            None => String::new(),
        };
        tx.execute(
            "INSERT INTO observation (kind, source, app, at_ms, text, place_id, event_id, sender, sender_person_id, dedupe_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![o.kind.as_str(), o.source, o.app, o.at_ms, o.text, place_id, o.event_id, who, o.person_id, key],
        )
        .map_err(|e| e.to_string())?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO observation_fts (rowid, text, who, where_) VALUES (?1, ?2, ?3, ?4)",
            params![id, o.text, who, o.place.clone().unwrap_or_default()],
        )
        .map_err(|e| e.to_string())?;
        note_source(&tx, &o.source, &o.app, o.at_ms)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Ingest::Inserted(id))
    }

    fn has_observation(&self, key: &str) -> Result<bool, String> {
        self.conn
            .query_row("SELECT 1 FROM observation WHERE dedupe_key = ?1", params![key], |_| Ok(()))
            .optional()
            .map(|r| r.is_some())
            .map_err(|e| e.to_string())
    }

    /// Store a given (calendar) or inferred event, or update the one already
    /// stored for the same OS id — a re-read of the calendar must not
    /// duplicate. Returns the event's id.
    pub(crate) fn upsert_event(&mut self, e: &NewEvent) -> Result<i64, String> {
        let key = if e.source_key.is_empty() {
            dedupe_key("event", &normalize_alias(&e.title), "", e.start_ms, "")
        } else {
            dedupe_key("event", &e.app, &e.source_key, 0, "")
        };
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let place_id = match &e.place {
            Some(name) => Some(place_id(&tx, name)?),
            None => None,
        };
        tx.execute(
            "INSERT INTO event (title, start_ms, end_ms, place_id, origin, app, source_key, dedupe_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(dedupe_key) DO UPDATE SET
               title = excluded.title, start_ms = excluded.start_ms, end_ms = excluded.end_ms,
               place_id = COALESCE(excluded.place_id, place_id)",
            params![e.title, e.start_ms, e.end_ms, place_id, e.origin.as_str(), e.app, e.source_key, key],
        )
        .map_err(|e| e.to_string())?;
        let id: i64 = tx
            .query_row("SELECT id FROM event WHERE dedupe_key = ?1", params![key], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// This person was at this event. A second call for the same pair
    /// updates the role.
    pub(crate) fn add_participant(&mut self, event_id: i64, person_id: i64, role: Role, source: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO participation (event_id, person_id, role, source) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(event_id, person_id) DO UPDATE SET role = excluded.role, source = excluded.source",
                params![event_id, person_id, role.as_str(), source],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// A person known only by name (a calendar attendee, a contact): find or
    /// create them, keyed by `app` + `key` like any sender.
    pub(crate) fn person_for(&mut self, app: &str, key: &str, name: &str) -> Result<i64, String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        let id = resolve_person(&tx, app, key, name)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Tie an observation to an event ("this photo was at the wedding").
    pub(crate) fn attach_to_event(&mut self, observation_id: i64, event_id: i64) -> Result<(), String> {
        self.conn
            .execute("UPDATE observation SET event_id = ?1 WHERE id = ?2", params![event_id, observation_id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    // ----- people ------------------------------------------------------------

    /// Who is `who`? Exact normalised alias first; then an alias that starts
    /// with it ("tunde" → "tunde bakare"); then a word of an alias ("bakare").
    /// Several matches at the same tier are returned for the user to choose
    /// from — never silently the first.
    pub(crate) fn find_person(&self, who: &str) -> Result<PersonMatch, String> {
        let norm = normalize_alias(who);
        if norm.is_empty() {
            return Ok(PersonMatch::None);
        }
        let tiers: [(&str, String); 3] = [
            ("alias_norm = ?1", norm.clone()),
            ("alias_norm LIKE ?1", format!("{}%", like_escape(&norm))),
            ("(' ' || alias_norm || ' ') LIKE ?1", format!("% {} %", like_escape(&norm))),
        ];
        for (cond, arg) in tiers {
            let sql = format!(
                "SELECT DISTINCT p.id, p.display_name,
                        (SELECT COUNT(*) FROM observation WHERE sender_person_id = p.id)
                 FROM person p JOIN person_alias a ON a.person_id = p.id
                 WHERE {cond} AND p.is_self = 0 ORDER BY 3 DESC, p.id"
            );
            let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let people: Vec<Person> = stmt
                .query_map(params![arg], row_to_person)
                .map_err(|e| e.to_string())?
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            match people.len() {
                0 => continue,
                1 => return Ok(PersonMatch::One(people.into_iter().next().unwrap())),
                _ => return Ok(PersonMatch::Ambiguous(people)),
            }
        }
        Ok(PersonMatch::None)
    }

    /// Everyone Verdure has heard from, most talkative first.
    pub(crate) fn people(&self) -> Result<Vec<Person>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.display_name, (SELECT COUNT(*) FROM observation WHERE sender_person_id = p.id) AS n
                 FROM person p WHERE p.is_self = 0 ORDER BY n DESC, p.display_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], row_to_person).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// The user says two rows are one person ("Tunde" on WhatsApp is "Tunde
    /// Bakare" in SMS): `other`'s aliases, identities, observations and
    /// participations move to `keep`. Merging is the user's call, never
    /// guessed (knowledge/25).
    pub(crate) fn merge_people(&mut self, keep: i64, other: i64) -> Result<(), String> {
        if keep == other {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for sql in [
            "UPDATE OR IGNORE person_alias SET person_id = ?1 WHERE person_id = ?2",
            "UPDATE OR IGNORE identity SET person_id = ?1 WHERE person_id = ?2",
            "UPDATE OR IGNORE participation SET person_id = ?1 WHERE person_id = ?2",
            "UPDATE observation SET sender_person_id = ?1 WHERE sender_person_id = ?2",
            "UPDATE facts SET subject_person_id = ?1 WHERE subject_person_id = ?2",
            "DELETE FROM person WHERE id = ?1 AND ?2 = ?2",
        ] {
            let args: [i64; 2] = if sql.starts_with("DELETE") { [other, keep] } else { [keep, other] };
            tx.execute(sql, params![args[0], args[1]]).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    // ----- lookups (structured first) ---------------------------------------

    /// "The last thing Tunde said": the newest `limit` observations from this
    /// person, newest first. An indexed lookup, no search involved.
    pub(crate) fn last_from(&self, person_id: i64, limit: usize) -> Result<Vec<Observation>, String> {
        self.query_observations(
            "WHERE o.sender_person_id = ?1 ORDER BY o.at_ms DESC LIMIT ?2",
            params![person_id, limit as i64],
        )
    }

    /// Observations in `[start_ms, end_ms)`, optionally from one person,
    /// newest first — every kind on one timeline. The caller resolves "last
    /// week" into the range (temporal parsing is a rule in Rust, never the
    /// model's job).
    pub(crate) fn in_range(&self, start_ms: i64, end_ms: i64, person_id: Option<i64>, limit: usize) -> Result<Vec<Observation>, String> {
        match person_id {
            Some(p) => self.query_observations(
                "WHERE o.at_ms >= ?1 AND o.at_ms < ?2 AND o.sender_person_id = ?3 ORDER BY o.at_ms DESC LIMIT ?4",
                params![start_ms, end_ms, p, limit as i64],
            ),
            None => self.query_observations(
                "WHERE o.at_ms >= ?1 AND o.at_ms < ?2 ORDER BY o.at_ms DESC LIMIT ?3",
                params![start_ms, end_ms, limit as i64],
            ),
        }
    }

    /// Events overlapping `[start_ms, end_ms)`, soonest first — "what am I
    /// doing on the 25th".
    pub(crate) fn events_in_range(&self, start_ms: i64, end_ms: i64, limit: usize) -> Result<Vec<Event>, String> {
        self.query_events(
            "WHERE e.start_ms < ?2 AND e.end_ms >= ?1 ORDER BY e.start_ms LIMIT ?3",
            params![start_ms, end_ms, limit as i64],
        )
    }

    /// Events this person was at, newest first — "when did I last see Ada".
    pub(crate) fn events_with(&self, person_id: i64, limit: usize) -> Result<Vec<Event>, String> {
        self.query_events(
            "JOIN participation pa ON pa.event_id = e.id WHERE pa.person_id = ?1 ORDER BY e.start_ms DESC LIMIT ?2",
            params![person_id, limit as i64],
        )
    }

    /// Events whose title contains every content word of `words`, newest first.
    pub(crate) fn events_titled(&self, words: &str, limit: usize) -> Result<Vec<Event>, String> {
        let terms = content_words(words);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let conds: Vec<String> = terms.iter().map(|_| "LOWER(e.title) LIKE ?".to_string()).collect();
        let sql = format!(
            "SELECT e.id, e.title, e.start_ms, e.end_ms, p.name, e.origin, e.app
             FROM event e LEFT JOIN place p ON p.id = e.place_id
             WHERE {} ORDER BY e.start_ms DESC LIMIT {}",
            conds.join(" AND "),
            limit
        );
        let args: Vec<String> = terms.iter().map(|t| format!("%{}%", like_escape(t))).collect();
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), row_to_event)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Who was at an event.
    pub(crate) fn people_at(&self, event_id: i64) -> Result<Vec<Participant>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.display_name, (SELECT COUNT(*) FROM observation WHERE sender_person_id = p.id), pa.role
                 FROM participation pa JOIN person p ON p.id = pa.person_id
                 WHERE pa.event_id = ?1 ORDER BY p.display_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![event_id], |r| Ok(Participant { person: row_to_person(r)?, role: r.get(3)? }))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// What was recorded at an event, oldest first.
    pub(crate) fn at_event(&self, event_id: i64, limit: usize) -> Result<Vec<Observation>, String> {
        self.query_observations("WHERE o.event_id = ?1 ORDER BY o.at_ms LIMIT ?2", params![event_id, limit as i64])
    }

    /// Keyword search over everything stored: FTS5 ranked, ties to the newest,
    /// optionally scoped to one person. The question is turned into an OR of
    /// its content words (`fts_query`), so "what did Tunde say about the rent"
    /// finds messages about rent without the model writing a query first.
    pub(crate) fn search(&self, question: &str, person_id: Option<i64>, limit: usize) -> Result<Vec<Observation>, String> {
        let q = fts_query(question);
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let scope = if person_id.is_some() { "AND o.sender_person_id = ?3" } else { "AND ?3 IS NULL" };
        let sql = format!(
            "SELECT {OBSERVATION_COLUMNS} FROM observation_fts f JOIN observation o ON o.id = f.rowid
             LEFT JOIN threads t ON t.id = o.thread_id LEFT JOIN place p ON p.id = o.place_id
             WHERE observation_fts MATCH ?1 {scope}
             ORDER BY bm25(observation_fts, 3.0, 2.0, 1.0), o.at_ms DESC
             LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![q, limit as i64, person_id], row_to_observation)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn query_observations(&self, tail: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Observation>, String> {
        let sql = format!("SELECT {OBSERVATION_COLUMNS} {OBSERVATION_FROM} {tail}");
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(args, row_to_observation).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn query_events(&self, tail: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Event>, String> {
        let sql = format!(
            "SELECT e.id, e.title, e.start_ms, e.end_ms, p.name, e.origin, e.app
             FROM event e LEFT JOIN place p ON p.id = e.place_id {tail}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(args, row_to_event).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    // ----- what Verdure knows -----------------------------------------------

    /// Conversations, most recent first.
    pub(crate) fn threads(&self, limit: usize) -> Result<Vec<ThreadSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.id, t.app, t.title, t.is_group, t.last_at_ms,
                        (SELECT COUNT(*) FROM observation WHERE thread_id = t.id),
                        COALESCE((SELECT text FROM observation WHERE thread_id = t.id ORDER BY at_ms DESC LIMIT 1), '')
                 FROM threads t ORDER BY t.last_at_ms DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(ThreadSummary {
                    id: r.get(0)?,
                    app: r.get(1)?,
                    title: r.get(2)?,
                    is_group: r.get::<_, i64>(3)? != 0,
                    last_at_ms: r.get(4)?,
                    items: r.get(5)?,
                    last_text: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// One conversation, oldest first, the last `limit` messages.
    pub(crate) fn thread_items(&self, thread_id: i64, limit: usize) -> Result<Vec<Observation>, String> {
        let mut items = self.query_observations(
            "WHERE o.thread_id = ?1 ORDER BY o.at_ms DESC LIMIT ?2",
            params![thread_id, limit as i64],
        )?;
        items.reverse();
        Ok(items)
    }

    /// What is held, per app — shown on "What Verdure knows". Every kind of
    /// observation counts; threads only exist for messages.
    pub(crate) fn counts(&self) -> Result<Vec<AppCount>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT o.app, COUNT(o.id), COUNT(DISTINCT o.thread_id)
                 FROM observation o GROUP BY o.app ORDER BY COUNT(o.id) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok(AppCount { app: r.get(0)?, items: r.get(1)?, threads: r.get(2)? }))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    // ----- forgetting ----------------------------------------------------------

    /// Forget one conversation entirely — messages, their index entries, facts.
    pub(crate) fn delete_thread(&mut self, thread_id: i64) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM observation_fts WHERE rowid IN (SELECT id FROM observation WHERE thread_id = ?1)",
            params![thread_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM threads WHERE id = ?1", params![thread_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Forget one person: everything from them (and index entries), their
    /// participations, and the person row.
    pub(crate) fn delete_person(&mut self, person_id: i64) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM observation_fts WHERE rowid IN (SELECT id FROM observation WHERE sender_person_id = ?1)",
            params![person_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM observation WHERE sender_person_id = ?1", params![person_id]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM person WHERE id = ?1", params![person_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Forget one event: its participations go with it; observations that
    /// pointed at it stay, unlinked.
    pub(crate) fn delete_event(&mut self, event_id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM event WHERE id = ?1", params![event_id])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Forget everything. The user's one-tap exit from the whole feature.
    pub(crate) fn delete_all(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "DELETE FROM observation_fts; DELETE FROM observation; DELETE FROM threads; DELETE FROM event;
                 DELETE FROM place; DELETE FROM person; DELETE FROM sources;",
            )
            .map_err(|e| e.to_string())
    }
}

fn note_source(tx: &Transaction<'_>, source: &str, app: &str, at_ms: i64) -> Result<(), String> {
    tx.execute(
        "INSERT INTO sources (kind, app, added_at_ms) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
        params![source, app, at_ms],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// A place row for this name, created on first sight. Names are matched in
/// their normalised form, so "The Wheatbaker" and "the wheatbaker" are one.
fn place_id(tx: &Transaction<'_>, name: &str) -> Result<i64, String> {
    let norm = normalize_alias(name);
    let norm = if norm.is_empty() { name.trim().to_lowercase() } else { norm };
    tx.execute(
        "INSERT INTO place (name, name_norm) VALUES (?1, ?2) ON CONFLICT(name_norm) DO NOTHING",
        params![name.trim(), norm],
    )
    .map_err(|e| e.to_string())?;
    tx.query_row("SELECT id FROM place WHERE name_norm = ?1", params![norm], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// The identity rule on ingest: the app's key for this sender if we've seen
/// it; else a person whose alias is exactly this name (so the same "Tunde"
/// across two WhatsApp threads is one person, and "Tunde" on WhatsApp and
/// "Tunde" on Telegram are one person too — the user can split them if that
/// is wrong); else a new person. Two different full names are never merged
/// by guesswork.
fn resolve_person(tx: &Transaction<'_>, app: &str, sender_key: &str, name: &str) -> Result<i64, String> {
    let key = if sender_key.is_empty() { name } else { sender_key };
    if let Some(id) = tx
        .query_row("SELECT person_id FROM identity WHERE app = ?1 AND source_key = ?2", params![app, key], |r| r.get::<_, i64>(0))
        .optional()
        .map_err(|e| e.to_string())?
    {
        return Ok(id);
    }
    let norm = normalize_alias(name);
    let by_alias: Option<i64> = if norm.is_empty() {
        None
    } else {
        tx.query_row("SELECT person_id FROM person_alias WHERE alias_norm = ?1 ORDER BY person_id LIMIT 1", params![norm], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?
    };
    let id = match by_alias {
        Some(id) => id,
        None => {
            tx.execute("INSERT INTO person (display_name) VALUES (?1)", params![if name.is_empty() { key } else { name }])
                .map_err(|e| e.to_string())?;
            let id = tx.last_insert_rowid();
            if !norm.is_empty() {
                tx.execute(
                    "INSERT OR IGNORE INTO person_alias (alias_norm, person_id, kind) VALUES (?1, ?2, 'name')",
                    params![norm, id],
                )
                .map_err(|e| e.to_string())?;
            }
            id
        }
    };
    tx.execute(
        "INSERT OR IGNORE INTO identity (app, source_key, person_id) VALUES (?1, ?2, ?3)",
        params![app, key, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// The form names are compared in: lower-case, one space between words, no
/// punctuation, no diacritics on the common Latin letters. "Tunde  Bakare!"
/// and "tunde bakare" are the same alias.
pub(crate) fn normalize_alias(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut space = true;
    for c in name.chars() {
        // Combining marks (the accent on "ẹ́" is a separate code point) are
        // dropped, not treated as separators — otherwise "Ṣẹ́gun" splits in two.
        if ('\u{0300}'..='\u{036f}').contains(&c) {
            continue;
        }
        let c = fold_diacritic(c);
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            space = false;
        } else if !space {
            out.push(' ');
            space = true;
        }
    }
    out.trim_end().to_string()
}

fn fold_diacritic(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'a',
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        'ẹ' | 'Ẹ' => 'e',
        'ọ' | 'Ọ' => 'o',
        'ṣ' | 'Ṣ' => 's',
        other => other,
    }
}

fn like_escape(s: &str) -> String {
    s.replace('%', "").replace('_', "")
}

fn row_to_person(r: &rusqlite::Row<'_>) -> rusqlite::Result<Person> {
    Ok(Person { id: r.get(0)?, display_name: r.get(1)?, items: r.get(2)? })
}

fn row_to_event(r: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let origin: String = r.get(5)?;
    Ok(Event {
        id: r.get(0)?,
        title: r.get(1)?,
        start_ms: r.get(2)?,
        end_ms: r.get(3)?,
        place: r.get(4)?,
        origin: if origin == "inferred" { "inferred" } else { "given" },
        app: r.get(6)?,
    })
}

fn row_to_observation(r: &rusqlite::Row<'_>) -> rusqlite::Result<Observation> {
    let kind: String = r.get(1)?;
    let direction: String = r.get(9)?;
    Ok(Observation {
        id: r.get(0)?,
        kind: Kind::parse(&kind).as_str(),
        app: r.get(2)?,
        at_ms: r.get(3)?,
        text: r.get(4)?,
        thread_id: r.get(5)?,
        thread_title: r.get(6)?,
        sender: r.get(7)?,
        sender_person_id: r.get(8)?,
        direction: Direction::parse(&direction).as_str(),
        place: r.get(10)?,
        event_id: r.get(11)?,
    })
}

/// A stable identity for one record, so a re-posted notification, a
/// re-imported export or a re-read calendar never stores it twice. FNV-1a
/// rather than `DefaultHasher` because the latter is not promised stable
/// across Rust versions, and this key lives in a file on a phone for years.
pub(crate) fn dedupe_key(app: &str, scope: &str, key: &str, at_ms: i64, text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for chunk in [app.as_bytes(), b"\0", scope.as_bytes(), b"\0", key.as_bytes(), b"\0", at_ms.to_string().as_bytes(), b"\0", text.as_bytes()] {
        for &b in chunk {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

/// Words FTS5 shouldn't be asked about: they are in every message and in
/// every question, and matching on them ranks noise.
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "in", "on", "at", "for", "is", "are", "was", "were", "be",
    "it", "its", "this", "that", "these", "those", "i", "me", "my", "you", "your", "we", "our", "he", "she",
    "they", "them", "his", "her", "do", "did", "does", "what", "when", "who", "where", "which", "how", "why",
    "about", "say", "said", "tell", "told", "ask", "asked", "message", "messages", "text", "texted", "someone",
    "anyone", "has", "have", "had", "with", "from", "any", "can", "could", "please", "there", "here", "up",
    "coming", "soon", "recently", "last", "again", "just", "know", "think", "like", "get", "got", "yes", "no",
];

/// The content words of a question: lower-cased, stopwords and one-letter
/// fragments dropped, each once.
fn content_words(question: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for raw in question.split(|c: char| !c.is_alphanumeric() && c != '\'') {
        let w = raw.trim_matches('\'').to_lowercase();
        if w.len() < 2 || STOPWORDS.contains(&w.as_str()) || words.contains(&w) {
            continue;
        }
        words.push(w);
    }
    words
}

/// Turn a question into an FTS5 query: its content words, each a prefix
/// match, OR-ed. Prefixes let "rent" find "rental" and "Tunde" find "Tunde's".
/// Nothing a user can type reaches FTS5 unquoted, so its syntax can't break.
pub(crate) fn fts_query(question: &str) -> String {
    content_words(question)
        .iter()
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// Stored observations as the prompt's sources (knowledge/24's block, reused):
/// the sender and thread stand where a page title stood, the app where the
/// domain did, and the "url" is Verdure's own link for the footer to open.
pub(crate) fn observations_as_snippets(items: &[Observation]) -> Vec<Snippet> {
    items
        .iter()
        .map(|o| {
            let title = match (o.kind, o.direction) {
                ("message", "out") => format!("You, in {}", o.thread_title),
                ("message", _) if o.thread_title == o.sender || o.thread_title.is_empty() => o.sender.clone(),
                ("message", _) => format!("{} in {}", o.sender, o.thread_title),
                ("call", _) if !o.sender.is_empty() => format!("Call with {}", o.sender),
                ("photo", _) => match &o.place {
                    Some(p) => format!("Photo at {p}"),
                    None => "Photo".to_string(),
                },
                ("visit", _) => match &o.place {
                    Some(p) => format!("At {p}"),
                    None => "A visit".to_string(),
                },
                (k, _) => {
                    let mut s = k.to_string();
                    if let Some(c) = s.get_mut(0..1) {
                        c.make_ascii_uppercase();
                    }
                    s
                }
            };
            let url = match o.thread_id {
                Some(t) => format!("verdure://thread/{t}"),
                None => format!("verdure://observation/{}", o.id),
            };
            Snippet {
                title,
                url,
                domain: app_label(&o.app).to_string(),
                date: Some(format_date(o.at_ms)),
                text: o.text.clone(),
            }
        })
        .collect()
}

/// Events as the prompt's sources: "Tunde's wedding, 2026-12-25, at The
/// Wheatbaker — with Tunde, Ada".
pub(crate) fn events_as_snippets(events: &[(Event, Vec<Participant>)]) -> Vec<Snippet> {
    events
        .iter()
        .map(|(e, people)| {
            let mut text = String::new();
            if let Some(p) = &e.place {
                text.push_str(&format!("At {p}. "));
            }
            if !people.is_empty() {
                text.push_str("With ");
                text.push_str(&people.iter().map(|p| p.person.display_name.as_str()).collect::<Vec<_>>().join(", "));
                text.push('.');
            }
            if format_date(e.start_ms) != format_date(e.end_ms) {
                text.push_str(&format!(" Until {}.", format_date(e.end_ms)));
            }
            Snippet {
                title: e.title.clone(),
                url: format!("verdure://event/{}", e.id),
                domain: if e.app.is_empty() { "Calendar".to_string() } else { app_label(&e.app).to_string() },
                date: Some(format_date(e.start_ms)),
                text: text.trim().to_string(),
            }
        })
        .collect()
}

/// The app's everyday name, for the footer. Unknown packages show as-is.
pub(crate) fn app_label(app: &str) -> &str {
    match app {
        "com.whatsapp" | "com.whatsapp.w4b" => "WhatsApp",
        "com.instagram.android" => "Instagram",
        "org.telegram.messenger" => "Telegram",
        "com.facebook.orca" => "Messenger",
        "com.snapchat.android" => "Snapchat",
        "com.twitter.android" => "X",
        "sms" | "com.google.android.apps.messaging" => "Messages",
        "com.google.android.calendar" | "calendar" => "Calendar",
        "photos" | "com.google.android.apps.photos" => "Photos",
        other => other,
    }
}

/// `YYYY-MM-DD` from unix milliseconds, UTC — the prompt wants a date it can
/// reason about, not a timestamp. (Howard Hinnant's civil-from-days.)
pub(crate) fn format_date(at_ms: i64) -> String {
    let days = at_ms.div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Unix milliseconds from a civil date and time, UTC. Inverse of `format_date`.
pub(crate) fn to_unix_ms(y: i64, m: i64, d: i64, hh: i64, mm: i64, ss: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days * 86_400 + hh * 3600 + mm * 60 + ss) * 1000
}

/// Parse a WhatsApp "Export chat" text file into messages — the one-time,
/// user-driven way to give Verdure a conversation's past (knowledge/25).
///
/// Handles the Android form (`12/05/2026, 21:41 - Tunde: hi`), the iOS form
/// (`[12/05/2026, 9:41:03 PM] Tunde: hi`), 12- and 24-hour clocks, multi-line
/// messages (a line that doesn't start a message continues the last one), and
/// system lines without a sender ("Messages and calls are end-to-end
/// encrypted…"), which are dropped. `me` is the user's own display name in the
/// export, so their side becomes `Direction::Out`.
///
/// Dates: exports carry no timezone and the day/month order follows the
/// phone's locale. A day above 12 anywhere in the file settles the order for
/// the whole file; otherwise day-first is assumed (`[OQ-99]`).
pub(crate) fn parse_whatsapp_export(text: &str, thread_title: &str, me: &str) -> Vec<NewMessage> {
    let header = Regex::new(
        r"^\[?(?P<a>\d{1,2})/(?P<b>\d{1,2})/(?P<y>\d{2,4}),?\s+(?P<h>\d{1,2}):(?P<mi>\d{2})(?::(?P<s>\d{2}))?\s?(?P<ap>[AaPp][Mm])?\]?\s?-?\s?(?P<rest>.*)$",
    )
    .expect("static pattern");
    // One pass to learn the file: who speaks (a group names more than one
    // other sender) and which date field is the day.
    let mut senders = std::collections::HashSet::new();
    let mut month_first = false;
    let mut day_first = false;
    for line in text.lines() {
        if let Some(c) = header.captures(line.trim_start_matches('\u{200e}')) {
            if let Some((who, _)) = c["rest"].split_once(": ") {
                if who != me {
                    senders.insert(who.to_string());
                }
            }
            let a: i64 = c["a"].parse().unwrap_or(1);
            let b: i64 = c["b"].parse().unwrap_or(1);
            if a > 12 {
                day_first = true;
            }
            if b > 12 {
                month_first = true;
            }
        }
    }
    let is_group = senders.len() > 1;
    let month_first = month_first && !day_first;
    let thread_key = format!("export:{}", thread_title.trim().to_lowercase());
    let mut items: Vec<NewMessage> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_start_matches('\u{200e}');
        let Some(c) = header.captures(line) else {
            // Continuation of the previous message.
            if let Some(last) = items.last_mut() {
                if !line.trim().is_empty() {
                    last.text.push('\n');
                    last.text.push_str(line);
                }
            }
            continue;
        };
        let Some((who, body)) = c["rest"].split_once(": ") else {
            continue; // a system line, no sender
        };
        let a: i64 = c["a"].parse().unwrap_or(1);
        let b: i64 = c["b"].parse().unwrap_or(1);
        let mut y: i64 = c["y"].parse().unwrap_or(2000);
        if y < 100 {
            y += 2000;
        }
        let (d, m) = if month_first { (b, a) } else { (a, b) };
        let mut hh: i64 = c["h"].parse().unwrap_or(0);
        let mi: i64 = c["mi"].parse().unwrap_or(0);
        let ss: i64 = c.name("s").map(|s| s.as_str().parse().unwrap_or(0)).unwrap_or(0);
        if let Some(ap) = c.name("ap") {
            let pm = ap.as_str().eq_ignore_ascii_case("pm");
            hh = match (hh, pm) {
                (12, false) => 0,
                (12, true) => 12,
                (h, true) => h + 12,
                (h, false) => h,
            };
        }
        let mine = who == me;
        items.push(NewMessage {
            source: source::CHAT_EXPORT.to_string(),
            app: "com.whatsapp".to_string(),
            thread_key: thread_key.clone(),
            thread_title: thread_title.to_string(),
            is_group,
            sender: if mine { String::new() } else { who.to_string() },
            sender_key: if mine { String::new() } else { who.to_string() },
            direction: if mine { Direction::Out } else { Direction::In },
            at_ms: to_unix_ms(y, m, d, hh, mi, ss),
            text: body.to_string(),
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(app: &str, thread: &str, sender: &str, at_ms: i64, text: &str) -> NewMessage {
        NewMessage {
            source: source::NOTIFICATION.to_string(),
            app: app.to_string(),
            thread_key: thread.to_lowercase(),
            thread_title: thread.to_string(),
            is_group: false,
            sender: sender.to_string(),
            sender_key: sender.to_string(),
            direction: if sender.is_empty() { Direction::Out } else { Direction::In },
            at_ms,
            text: text.to_string(),
        }
    }

    fn calendar(title: &str, start_ms: i64, end_ms: i64, place: Option<&str>, key: &str) -> NewEvent {
        NewEvent {
            title: title.to_string(),
            start_ms,
            end_ms,
            place: place.map(str::to_string),
            origin: Origin::Given,
            app: "com.google.android.calendar".to_string(),
            source_key: key.to_string(),
        }
    }

    fn seeded() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "Tunde", 1_000, "Rent is due on the 5th, landlord wants 1.2m")).unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "", 2_000, "Ok I'll send my half on Friday")).unwrap();
        s.ingest(&msg("org.telegram.messenger", "Ada", "Ada", 3_000, "Happy to help with the visa forms")).unwrap();
        s.ingest(&msg("sms", "+2348031234567", "GTBank", 4_000, "Acct debited NGN 45,000.00")).unwrap();
        s.ingest(&msg("com.whatsapp", "Tunde", "Tunde", 5_000, "See you Friday then")).unwrap();
        s
    }

    fn one(m: PersonMatch) -> Person {
        match m {
            PersonMatch::One(p) => p,
            other => panic!("expected one person, got {other:?}"),
        }
    }

    #[test]
    fn the_schema_builds_and_fts5_is_available() {
        // `bundled` must bring FTS5; if it ever doesn't, this is the first to fail.
        let s = Store::open_in_memory().unwrap();
        let v: i64 = s.conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 1);
        assert!(s.search("anything", None, 5).unwrap().is_empty());
        assert!(s.events_in_range(0, i64::MAX, 5).unwrap().is_empty());
    }

    #[test]
    fn ingest_stores_once_and_recognises_a_repost() {
        let mut s = Store::open_in_memory().unwrap();
        let m = msg("com.whatsapp", "Tunde", "Tunde", 1_000, "hello");
        assert!(matches!(s.ingest(&m).unwrap(), Ingest::Inserted(_)));
        // A notification update re-posts the same message: not stored again.
        assert_eq!(s.ingest(&m).unwrap(), Ingest::Duplicate);
        // The same words at a different time ARE a new message.
        let later = NewMessage { at_ms: 5_000, ..m.clone() };
        assert!(matches!(s.ingest(&later).unwrap(), Ingest::Inserted(_)));
        assert_eq!(s.counts().unwrap()[0].items, 2);
    }

    #[test]
    fn the_last_thing_someone_said_is_a_lookup_not_a_search() {
        let s = seeded();
        let tunde = one(s.find_person("Tunde").unwrap());
        let last = s.last_from(tunde.id, 1).unwrap();
        assert_eq!(last[0].text, "See you Friday then");
        assert_eq!(last[0].kind, "message");
        // The user's own replies are not "what Tunde said".
        assert!(s.last_from(tunde.id, 10).unwrap().iter().all(|i| i.direction == "in"));
        assert_eq!(tunde.items, 2);
    }

    #[test]
    fn people_resolve_by_exact_prefix_and_word_and_stay_apart_when_unsure() {
        let mut s = seeded();
        s.ingest(&msg("sms", "+2348031111111", "Tunde Bakare", 6_000, "Call me")).unwrap();
        s.ingest(&msg("com.whatsapp", "Jackson Lee", "Jackson Lee", 7_000, "hey")).unwrap();
        s.ingest(&msg("com.whatsapp", "Jackson Okoro", "Jackson Okoro", 8_000, "yo")).unwrap();
        // Exact alias wins outright even though a longer alias also starts with it.
        assert_eq!(one(s.find_person("tunde").unwrap()).display_name, "Tunde");
        // A prefix with one candidate resolves; a word inside an alias resolves.
        assert_eq!(one(s.find_person("Tunde B").unwrap()).display_name, "Tunde Bakare");
        assert_eq!(one(s.find_person("Bakare").unwrap()).display_name, "Tunde Bakare");
        assert_eq!(one(s.find_person("Okoro").unwrap()).display_name, "Jackson Okoro");
        // Two Jacksons: a question back to the user, never a guess.
        match s.find_person("Jackson").unwrap() {
            PersonMatch::Ambiguous(ps) => assert_eq!(ps.len(), 2),
            other => panic!("{other:?}"),
        }
        assert_eq!(s.find_person("Nobody").unwrap(), PersonMatch::None);
        assert_eq!(s.find_person("  ").unwrap(), PersonMatch::None);
    }

    #[test]
    fn the_same_name_in_two_apps_is_one_person_until_the_user_says_otherwise() {
        let mut s = Store::open_in_memory().unwrap();
        s.ingest(&msg("com.whatsapp", "Ada", "Ada", 1_000, "wa")).unwrap();
        s.ingest(&msg("org.telegram.messenger", "Ada", "Ada", 2_000, "tg")).unwrap();
        let ada = one(s.find_person("Ada").unwrap());
        assert_eq!(ada.items, 2, "the same display name resolves to one person across apps");
        assert_eq!(s.people().unwrap().len(), 1);
        // A different name is a different person; merging them is explicit.
        s.ingest(&msg("sms", "+234111", "Ada Obi", 3_000, "sms")).unwrap();
        assert_eq!(s.people().unwrap().len(), 2);
        let full = one(s.find_person("Ada Obi").unwrap());
        // Participation follows the merge too.
        let ev = s.upsert_event(&calendar("Lunch", 10_000, 11_000, None, "ev-1")).unwrap();
        s.add_participant(ev, full.id, Role::Invited, source::CALENDAR).unwrap();
        s.merge_people(ada.id, full.id).unwrap();
        assert_eq!(s.people().unwrap().len(), 1);
        assert_eq!(s.last_from(ada.id, 10).unwrap().len(), 3);
        assert_eq!(s.events_with(ada.id, 5).unwrap().len(), 1);
        // After the merge the SMS key is this person's too: new SMS lands on it.
        s.ingest(&msg("sms", "+234111", "Ada Obi", 4_000, "again")).unwrap();
        assert_eq!(s.last_from(ada.id, 10).unwrap().len(), 4);
    }

    #[test]
    fn a_time_range_optionally_scoped_to_a_person() {
        let s = seeded();
        let all = s.in_range(2_000, 5_000, None, 10).unwrap();
        assert_eq!(all.iter().map(|i| i.at_ms).collect::<Vec<_>>(), [4_000, 3_000, 2_000], "newest first, end exclusive");
        let tunde = one(s.find_person("Tunde").unwrap());
        let his = s.in_range(0, 10_000, Some(tunde.id), 10).unwrap();
        assert_eq!(his.len(), 2);
    }

    #[test]
    fn every_kind_shares_one_timeline() {
        let mut s = seeded();
        let tunde = one(s.find_person("Tunde").unwrap());
        let call = NewObservation {
            kind: Kind::Call,
            source: source::CALL_LOG.to_string(),
            app: "com.google.android.dialer".to_string(),
            source_key: "call-77".to_string(),
            at_ms: 3_500,
            text: "Missed call".to_string(),
            person_id: Some(tunde.id),
            place: None,
            event_id: None,
        };
        assert!(matches!(s.record(&call).unwrap(), Ingest::Inserted(_)));
        assert_eq!(s.record(&call).unwrap(), Ingest::Duplicate, "a rescan of the call log stores nothing");
        let photo = NewObservation {
            kind: Kind::Photo,
            source: source::PHOTOS.to_string(),
            app: "photos".to_string(),
            source_key: "media-123".to_string(),
            at_ms: 4_500,
            text: String::new(),
            person_id: None,
            place: Some("The Wheatbaker".to_string()),
            event_id: None,
        };
        s.record(&photo).unwrap();
        // One query, every kind, in time order.
        let all = s.in_range(3_000, 5_000, None, 10).unwrap();
        assert_eq!(all.iter().map(|o| o.kind).collect::<Vec<_>>(), ["photo", "message", "call", "message"]);
        assert_eq!(all[0].place.as_deref(), Some("The Wheatbaker"));
        // The call is "from Tunde" like a message is.
        assert_eq!(s.last_from(tunde.id, 1).unwrap()[0].text, "See you Friday then");
        assert!(s.last_from(tunde.id, 10).unwrap().iter().any(|o| o.kind == "call"));
        // A photo with no words is still findable by where it was taken.
        let hits = s.search("wheatbaker", None, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "photo");
        // The snippets describe each kind in words.
        let snips = observations_as_snippets(&all);
        assert_eq!(snips[0].title, "Photo at The Wheatbaker");
        assert_eq!(snips[2].title, "Call with Tunde");
        assert!(snips[0].url.starts_with("verdure://observation/"));
        // Every kind is counted for transparency; threads only for messages.
        let counts = s.counts().unwrap();
        assert_eq!(counts.iter().find(|c| c.app == "photos").unwrap().threads, 0);
    }

    #[test]
    fn events_link_people_to_a_time_and_a_place() {
        let mut s = seeded();
        let tunde = one(s.find_person("Tunde").unwrap());
        let ada = one(s.find_person("Ada").unwrap());
        let wedding = s
            .upsert_event(&calendar("Tunde's wedding", 20_000, 30_000, Some("The Wheatbaker"), "cal-9"))
            .unwrap();
        s.add_participant(wedding, tunde.id, Role::Organiser, source::CALENDAR).unwrap();
        s.add_participant(wedding, ada.id, Role::Invited, source::CALENDAR).unwrap();
        // A second read of the calendar updates, never duplicates.
        let again = s
            .upsert_event(&calendar("Tunde's wedding (moved)", 21_000, 30_000, None, "cal-9"))
            .unwrap();
        assert_eq!(again, wedding);
        let on_the_day = s.events_in_range(25_000, 26_000, 5).unwrap();
        assert_eq!(on_the_day.len(), 1);
        assert_eq!(on_the_day[0].title, "Tunde's wedding (moved)");
        assert_eq!(on_the_day[0].start_ms, 21_000);
        assert_eq!(on_the_day[0].place.as_deref(), Some("The Wheatbaker"), "a missing place on re-read keeps the old one");
        assert!(s.events_in_range(31_000, 40_000, 5).unwrap().is_empty());
        // Who was there, and what each person was at.
        let people = s.people_at(wedding).unwrap();
        assert_eq!(people.iter().map(|p| (p.person.display_name.as_str(), p.role.as_str())).collect::<Vec<_>>(),
                   [("Ada", "invited"), ("Tunde", "organiser")]);
        assert_eq!(s.events_with(ada.id, 5).unwrap()[0].id, wedding);
        assert_eq!(s.events_titled("wedding", 5).unwrap().len(), 1);
        assert!(s.events_titled("the of", 5).unwrap().is_empty());
        // A message tied to the event shows up under it.
        let said = s.last_from(tunde.id, 1).unwrap()[0].id;
        s.attach_to_event(said, wedding).unwrap();
        assert_eq!(s.at_event(wedding, 5).unwrap()[0].text, "See you Friday then");
        // The prompt sees the event in words.
        let snips = events_as_snippets(&[(on_the_day[0].clone(), people)]);
        assert_eq!(snips[0].domain, "Calendar");
        assert_eq!(snips[0].text, "At The Wheatbaker. With Ada, Tunde.");
        assert_eq!(snips[0].url, format!("verdure://event/{wedding}"));
    }

    #[test]
    fn an_inferred_event_dedupes_on_its_title_and_start() {
        let mut s = Store::open_in_memory().unwrap();
        let inferred = NewEvent {
            title: "Dinner at Ada's".to_string(),
            start_ms: 1_000,
            end_ms: 2_000,
            place: Some("Ada's".to_string()),
            origin: Origin::Inferred,
            app: String::new(),
            source_key: String::new(),
        };
        let a = s.upsert_event(&inferred).unwrap();
        let b = s.upsert_event(&NewEvent { title: "dinner at ADA'S".to_string(), ..inferred.clone() }).unwrap();
        assert_eq!(a, b);
        let c = s.upsert_event(&NewEvent { start_ms: 9_000, ..inferred.clone() }).unwrap();
        assert_ne!(a, c);
        assert_eq!(s.events_in_range(0, 10_000, 5).unwrap()[0].origin, "inferred");
    }

    #[test]
    fn deleting_an_event_forgets_who_was_there_but_not_the_people() {
        let mut s = seeded();
        let ada = one(s.find_person("Ada").unwrap());
        let ev = s.upsert_event(&calendar("Lunch", 1_000, 2_000, None, "ev-2")).unwrap();
        s.add_participant(ev, ada.id, Role::Attended, source::USER).unwrap();
        let note_id = match s.record(&NewObservation {
            kind: Kind::Note,
            source: source::USER.to_string(),
            app: "verdure".to_string(),
            source_key: "n1".to_string(),
            at_ms: 1_500,
            text: "Ada paid".to_string(),
            person_id: None,
            place: None,
            event_id: Some(ev),
        }) {
            Ok(Ingest::Inserted(id)) => id,
            other => panic!("{other:?}"),
        };
        s.delete_event(ev).unwrap();
        assert!(s.events_with(ada.id, 5).unwrap().is_empty());
        assert_eq!(s.find_person("Ada").unwrap(), PersonMatch::One(ada));
        // The note survives, unlinked.
        let note = s.in_range(1_500, 1_501, None, 1).unwrap();
        assert_eq!(note[0].id, note_id);
        assert_eq!(note[0].event_id, None);
    }

    #[test]
    fn search_finds_by_words_sender_and_thread_with_prefixes() {
        let s = seeded();
        let hits = s.search("what did Tunde say about the rent?", None, 5).unwrap();
        assert_eq!(hits[0].text, "Rent is due on the 5th, landlord wants 1.2m");
        // "Tunde" matches the thread title and sender; the user's own reply in
        // the Tunde thread is found through the thread title.
        assert!(hits.iter().any(|h| h.direction == "out"));
        // Scoped to a person, only their items.
        let tunde = one(s.find_person("Tunde").unwrap());
        let scoped = s.search("friday", Some(tunde.id), 5).unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].text, "See you Friday then");
        // Prefix: "visa" finds "visa forms"; "debit" finds "debited".
        assert_eq!(s.search("visa", None, 5).unwrap()[0].app, "org.telegram.messenger");
        assert_eq!(s.search("debit", None, 5).unwrap()[0].sender, "GTBank");
        // Stopwords alone are no query at all.
        assert!(s.search("what did they say?", None, 5).unwrap().is_empty());
    }

    #[test]
    fn fts_query_quotes_everything_and_drops_noise() {
        assert_eq!(fts_query("What did Tunde say about the rent?"), "\"tunde\"* OR \"rent\"*");
        assert_eq!(fts_query("whose birthday is coming up soon"), "\"whose\"* OR \"birthday\"*");
        // Dangerous FTS syntax is neutralised by quoting.
        assert_eq!(fts_query("rent\" OR NEAR("), "\"rent\"* OR \"near\"*");
        assert_eq!(fts_query("the of and"), "");
    }

    #[test]
    fn aliases_normalise_case_spacing_punctuation_and_diacritics() {
        assert_eq!(normalize_alias("  Tunde   Bakare! "), "tunde bakare");
        assert_eq!(normalize_alias("Adébáyọ̀"), "adebayo");
        assert_eq!(normalize_alias("Ọlá-Ṣẹ́gun"), "ola segun");
        assert_eq!(normalize_alias("+234 803 123 4567"), "234 803 123 4567");
        assert_eq!(normalize_alias("!!!"), "");
    }

    #[test]
    fn threads_list_newest_first_with_counts_and_last_line() {
        let s = seeded();
        let t = s.threads(10).unwrap();
        assert_eq!(t[0].title, "Tunde");
        let tunde = &t[0];
        assert_eq!(tunde.items, 3);
        assert_eq!(tunde.last_text, "See you Friday then");
        assert_eq!(tunde.last_at_ms, 5_000);
        let items = s.thread_items(tunde.id, 10).unwrap();
        assert_eq!(items.iter().map(|i| i.at_ms).collect::<Vec<_>>(), [1_000, 2_000, 5_000], "oldest first");
    }

    #[test]
    fn deleting_forgets_everywhere() {
        let mut s = seeded();
        let tunde_thread = s.threads(10).unwrap().into_iter().find(|x| x.title == "Tunde").unwrap();
        s.delete_thread(tunde_thread.id).unwrap();
        assert!(s.search("rent", None, 5).unwrap().is_empty(), "the index forgets too");
        assert!(s.threads(10).unwrap().iter().all(|t| t.title != "Tunde"));
        // The person row survives a thread deletion (they may be in other
        // threads); deleting the person removes their items everywhere.
        let ada = one(s.find_person("Ada").unwrap());
        s.delete_person(ada.id).unwrap();
        assert_eq!(s.find_person("Ada").unwrap(), PersonMatch::None);
        assert!(s.search("visa", None, 5).unwrap().is_empty());
        s.upsert_event(&calendar("Lunch", 1, 2, Some("Somewhere"), "x")).unwrap();
        s.delete_all().unwrap();
        assert!(s.threads(10).unwrap().is_empty());
        assert!(s.people().unwrap().is_empty());
        assert!(s.counts().unwrap().is_empty());
        assert!(s.events_in_range(0, 10, 5).unwrap().is_empty());
    }

    #[test]
    fn dedupe_key_is_stable_and_sensitive() {
        let k = dedupe_key("com.whatsapp", "tunde", "Tunde", 1_000, "hi");
        assert_eq!(k, dedupe_key("com.whatsapp", "tunde", "Tunde", 1_000, "hi"));
        assert_ne!(k, dedupe_key("com.whatsapp", "tunde", "Tunde", 1_001, "hi"));
        assert_ne!(k, dedupe_key("com.whatsapp", "tunde", "Ada", 1_000, "hi"));
        assert_eq!(k.len(), 16);
    }

    #[test]
    fn dates_round_trip() {
        let ms = to_unix_ms(2026, 10, 21, 9, 41, 3);
        assert_eq!(format_date(ms), "2026-10-21");
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(to_unix_ms(2024, 2, 29, 0, 0, 0)), "2024-02-29");
    }

    #[test]
    fn observations_become_sources_the_prompt_already_understands() {
        let s = seeded();
        let hits = s.search("rent", None, 5).unwrap();
        let snippets = observations_as_snippets(&hits);
        assert_eq!(snippets[0].domain, "WhatsApp");
        assert_eq!(snippets[0].title, "Tunde");
        assert_eq!(snippets[0].date.as_deref(), Some("1970-01-01"));
        assert!(snippets[0].url.starts_with("verdure://thread/"));
        let block = crate::web::sources_block("rent", &snippets);
        assert!(block.contains("[1] WhatsApp, 1970-01-01 — Tunde\nRent is due"));
    }

    const ANDROID_EXPORT: &str = "\
12/05/2026, 21:41 - Messages and calls are end-to-end encrypted. No one outside of this chat can read them.
12/05/2026, 21:41 - Tunde: Rent is due on the 5th
landlord wants 1.2m
12/05/2026, 21:43 - Ge: Ok I'll send my half on Friday
13/05/2026, 08:02 - Tunde: 🙏🏾
";

    const IOS_EXPORT: &str = "\
[05/12/2026, 9:41:03 PM] Tunde: Rent is due on the 5th
[05/12/2026, 9:43:10 PM] Ge: Ok I'll send my half on Friday
[05/13/2026, 12:02:00 AM] Tunde: 🙏🏾
";

    #[test]
    fn parses_an_android_export_with_multiline_and_system_lines() {
        let items = parse_whatsapp_export(ANDROID_EXPORT, "Tunde", "Ge");
        assert_eq!(items.len(), 3, "the system line is dropped");
        assert_eq!(items[0].sender, "Tunde");
        assert_eq!(items[0].text, "Rent is due on the 5th\nlandlord wants 1.2m");
        assert_eq!(items[0].direction, Direction::In);
        assert_eq!(items[1].direction, Direction::Out);
        assert_eq!(items[1].sender, "", "the user's side carries no sender name");
        assert_eq!(format_date(items[0].at_ms), "2026-05-12");
        assert_eq!(format_date(items[2].at_ms), "2026-05-13");
        assert!(!items[0].is_group, "one other sender → a 1:1 chat");
        assert_eq!(items[0].source, source::CHAT_EXPORT);
    }

    #[test]
    fn parses_an_ios_export_and_learns_the_date_order_from_the_file() {
        let items = parse_whatsapp_export(IOS_EXPORT, "Tunde", "Ge");
        assert_eq!(items.len(), 3);
        // 05/13 can only be month-first, which settles 05/12 for the whole
        // file: 12 May, not 5 December.
        assert_eq!(format_date(items[0].at_ms), "2026-05-12");
        assert_eq!(format_date(items[2].at_ms), "2026-05-13");
        // 9:41 PM → 21:41; 12:02 AM → 00:02.
        assert_eq!((items[0].at_ms / 3_600_000) % 24, 21);
        assert_eq!((items[2].at_ms / 3_600_000) % 24, 0);
        // A file with every day ≤ 12 stays ambiguous and is read day-first ([OQ-99]).
        let ambiguous = parse_whatsapp_export("[05/12/2026, 9:41:03 PM] Tunde: hi\n", "Tunde", "Ge");
        assert_eq!(format_date(ambiguous[0].at_ms), "2026-12-05");
    }

    #[test]
    fn a_group_export_is_marked_as_a_group() {
        let text = "12/05/2026, 21:41 - Tunde: hi\n12/05/2026, 21:42 - Ada: hello\n12/05/2026, 21:43 - Ge: hey\n";
        let items = parse_whatsapp_export(text, "Flatmates", "Ge");
        assert!(items.iter().all(|i| i.is_group));
        assert_eq!(items[2].direction, Direction::Out);
    }

    #[test]
    fn an_export_imports_without_duplicates_and_is_searchable() {
        let mut s = Store::open_in_memory().unwrap();
        for i in parse_whatsapp_export(ANDROID_EXPORT, "Tunde", "Ge") {
            assert!(matches!(s.ingest(&i).unwrap(), Ingest::Inserted(_)));
        }
        for i in parse_whatsapp_export(ANDROID_EXPORT, "Tunde", "Ge") {
            assert_eq!(s.ingest(&i).unwrap(), Ingest::Duplicate, "re-sharing the same file stores nothing");
        }
        let hits = s.search("landlord", None, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("landlord"));
        let tunde = one(s.find_person("Tunde").unwrap());
        assert_eq!(s.last_from(tunde.id, 1).unwrap()[0].text, "🙏🏾");
    }
}
