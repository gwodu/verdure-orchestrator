//! The personal context store (knowledge/25): what Verdure has seen, kept on
//! the device so it can answer "what did Tunde say about the rent?" the way it
//! answers from the web — from retrieved sources, cited, tappable.
//!
//! One SQLite file in the app's private folder. Not a "notifications table":
//! a notification is a transport, and what it carried is an **item** — a
//! message from a person in a thread at a time — the same shape whether it
//! arrived as an Android notification, an SMS, or a chat export the user
//! shared in.
//!
//! The store is **entity-centric and structured first** (knowledge/25, from
//! the founder's storage research): people are rows, every item points at the
//! person who sent it, and the questions that matter most — "the last thing
//! Tunde said", "what did Ada send last week" — are indexed lookups, not
//! retrieval. Keyword search (FTS5) is the second tier, for "the rent"; vectors
//! are a later, optional third. Small models ignore or are misled by retrieved
//! text far more than they are helped by it (Pandey et al. 2026, arXiv
//! 2603.11513), so the fewer and more exact the records, the better.
//!
//! Nothing here leaves the device. There is no sync, no relay, no server. The
//! file is excluded from cloud backup by the platform build (knowledge/25 —
//! the rule that keeps the promise), and the user can see and delete all of
//! it from "What Verdure knows".
//!
//! Wired into chat and the UI in step 2 of knowledge/25; until then this is
//! the store, its lookups and search, and the export parser, all tested here.
#![allow(dead_code)]

use std::path::Path;

use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::web::Snippet;

/// Schema version. This is the first version that has ever existed on a
/// device; from here on, migrations are appended with a version check and
/// never edited in place — a store on a phone has to survive every version of
/// this schema that ever shipped.
const SCHEMA_VERSION: i64 = 1;

/// Where an item came from. Strings, not an enum, because new sources are
/// added by the platform code that produces them, not by this module.
pub(crate) mod source {
    pub const NOTIFICATION: &str = "notification";
    pub const SMS: &str = "sms";
    pub const CHAT_EXPORT: &str = "chat_export";
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

/// One message as a source hands it in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewItem {
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

/// What `ingest` did. Notifications re-post the whole conversation every
/// time they update, so most calls are duplicates — that is normal, not an error.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ingest {
    Inserted(i64),
    Duplicate,
}

/// A stored item, as the UI and the prompt see it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Item {
    pub id: i64,
    pub thread_id: i64,
    pub app: String,
    pub thread_title: String,
    pub sender: String,
    pub sender_person_id: Option<i64>,
    pub direction: &'static str,
    pub at_ms: i64,
    pub text: String,
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
    /// How many items they sent.
    pub items: i64,
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
                    -- The entity layer: a person, the names they go by, and the
                    -- per-app keys that are them. Two tables with foreign keys
                    -- are the whole 'knowledge graph' v1 needs.
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
                    CREATE TABLE IF NOT EXISTS threads (
                        id INTEGER PRIMARY KEY,
                        app TEXT NOT NULL,
                        thread_key TEXT NOT NULL,
                        title TEXT NOT NULL,
                        is_group INTEGER NOT NULL DEFAULT 0,
                        last_at_ms INTEGER NOT NULL DEFAULT 0,
                        UNIQUE (app, thread_key)
                    );
                    CREATE TABLE IF NOT EXISTS items (
                        id INTEGER PRIMARY KEY,
                        thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
                        source TEXT NOT NULL,
                        sender TEXT NOT NULL,
                        sender_person_id INTEGER REFERENCES person(id) ON DELETE SET NULL,
                        direction TEXT NOT NULL,
                        at_ms INTEGER NOT NULL,
                        text TEXT NOT NULL,
                        dedupe_key TEXT NOT NULL UNIQUE
                    );
                    CREATE INDEX IF NOT EXISTS items_thread_at ON items(thread_id, at_ms);
                    CREATE INDEX IF NOT EXISTS items_sender_at ON items(sender_person_id, at_ms);
                    CREATE INDEX IF NOT EXISTS items_at ON items(at_ms);
                    -- Keyword tier. The index keeps its own copy of the text:
                    -- a contentless FTS5 table can't delete by rowid, and
                    -- messages are small (the double-storage concern in the
                    -- research is an email-scale one).
                    CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
                        text, sender, thread_title,
                        tokenize='unicode61 remove_diacritics 2'
                    );
                    -- Extracted later, while charging; every fact points at
                    -- its item, and facts are invalidated, not deleted.
                    CREATE TABLE IF NOT EXISTS facts (
                        id INTEGER PRIMARY KEY,
                        item_id INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
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

    /// Store one item, or recognise it as already stored. The thread and the
    /// sender's person row are created on first sight.
    pub(crate) fn ingest(&mut self, item: &NewItem) -> Result<Ingest, String> {
        let key = dedupe_key(&item.app, &item.thread_key, &item.sender_key, item.at_ms, &item.text);
        let exists: Option<i64> = self
            .conn
            .query_row("SELECT id FROM items WHERE dedupe_key = ?1", params![key], |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_some() {
            return Ok(Ingest::Duplicate);
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO threads (app, thread_key, title, is_group, last_at_ms) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(app, thread_key) DO UPDATE SET
               title = CASE WHEN excluded.title != '' THEN excluded.title ELSE title END,
               is_group = MAX(is_group, excluded.is_group),
               last_at_ms = MAX(last_at_ms, excluded.last_at_ms)",
            params![item.app, item.thread_key, item.thread_title, item.is_group as i64, item.at_ms],
        )
        .map_err(|e| e.to_string())?;
        let thread_id: i64 = tx
            .query_row(
                "SELECT id FROM threads WHERE app = ?1 AND thread_key = ?2",
                params![item.app, item.thread_key],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let person_id = match item.direction {
            Direction::Out => None,
            Direction::In => Some(resolve_person(&tx, &item.app, &item.sender_key, &item.sender)?),
        };
        tx.execute(
            "INSERT INTO items (thread_id, source, sender, sender_person_id, direction, at_ms, text, dedupe_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![thread_id, item.source, item.sender, person_id, item.direction.as_str(), item.at_ms, item.text, key],
        )
        .map_err(|e| e.to_string())?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO items_fts (rowid, text, sender, thread_title) VALUES (?1, ?2, ?3, ?4)",
            params![id, item.text, item.sender, item.thread_title],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO sources (kind, app, added_at_ms) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            params![item.source, item.app, item.at_ms],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Ingest::Inserted(id))
    }

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
                        (SELECT COUNT(*) FROM items WHERE sender_person_id = p.id)
                 FROM person p JOIN person_alias a ON a.person_id = p.id
                 WHERE {cond} AND p.is_self = 0 ORDER BY 3 DESC, p.id"
            );
            let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let people: Vec<Person> = stmt
                .query_map(params![arg], |r| Ok(Person { id: r.get(0)?, display_name: r.get(1)?, items: r.get(2)? }))
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

    /// "The last thing Tunde said": the newest `limit` items this person sent,
    /// newest first. An indexed lookup, no search involved (knowledge/25).
    pub(crate) fn last_from(&self, person_id: i64, limit: usize) -> Result<Vec<Item>, String> {
        self.query_items(
            "WHERE i.sender_person_id = ?1 ORDER BY i.at_ms DESC LIMIT ?2",
            params![person_id, limit as i64],
        )
    }

    /// Items in `[start_ms, end_ms)`, optionally from one person, newest first.
    /// The caller resolves "last week" into the range (temporal parsing is a
    /// rule in Rust, never the model's job).
    pub(crate) fn in_range(&self, start_ms: i64, end_ms: i64, person_id: Option<i64>, limit: usize) -> Result<Vec<Item>, String> {
        match person_id {
            Some(p) => self.query_items(
                "WHERE i.at_ms >= ?1 AND i.at_ms < ?2 AND i.sender_person_id = ?3 ORDER BY i.at_ms DESC LIMIT ?4",
                params![start_ms, end_ms, p, limit as i64],
            ),
            None => self.query_items(
                "WHERE i.at_ms >= ?1 AND i.at_ms < ?2 ORDER BY i.at_ms DESC LIMIT ?3",
                params![start_ms, end_ms, limit as i64],
            ),
        }
    }

    /// Keyword search over everything stored: FTS5 ranked, ties to the newest,
    /// optionally scoped to one person. The question is turned into an OR of
    /// its content words (`fts_query`), so "what did Tunde say about the rent"
    /// finds messages about rent without the model writing a query first.
    pub(crate) fn search(&self, question: &str, person_id: Option<i64>, limit: usize) -> Result<Vec<Item>, String> {
        let q = fts_query(question);
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let scope = if person_id.is_some() { "AND i.sender_person_id = ?3" } else { "AND ?3 IS NULL" };
        let sql = format!(
            "SELECT i.id, i.thread_id, t.app, t.title, i.sender, i.sender_person_id, i.direction, i.at_ms, i.text
             FROM items_fts f
             JOIN items i ON i.id = f.rowid
             JOIN threads t ON t.id = i.thread_id
             WHERE items_fts MATCH ?1 {scope}
             ORDER BY bm25(items_fts, 3.0, 2.0, 1.0), i.at_ms DESC
             LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![q, limit as i64, person_id], row_to_item)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn query_items(&self, tail: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Item>, String> {
        let sql = format!(
            "SELECT i.id, i.thread_id, t.app, t.title, i.sender, i.sender_person_id, i.direction, i.at_ms, i.text
             FROM items i JOIN threads t ON t.id = i.thread_id {tail}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(args, row_to_item).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Conversations, most recent first.
    pub(crate) fn threads(&self, limit: usize) -> Result<Vec<ThreadSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.id, t.app, t.title, t.is_group, t.last_at_ms,
                        (SELECT COUNT(*) FROM items WHERE thread_id = t.id),
                        COALESCE((SELECT text FROM items WHERE thread_id = t.id ORDER BY at_ms DESC LIMIT 1), '')
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

    /// One conversation, oldest first, the last `limit` items.
    pub(crate) fn thread_items(&self, thread_id: i64, limit: usize) -> Result<Vec<Item>, String> {
        let mut items = self.query_items(
            "WHERE i.thread_id = ?1 ORDER BY i.at_ms DESC LIMIT ?2",
            params![thread_id, limit as i64],
        )?;
        items.reverse();
        Ok(items)
    }

    /// Everyone Verdure has heard from, most talkative first.
    pub(crate) fn people(&self) -> Result<Vec<Person>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.display_name, (SELECT COUNT(*) FROM items WHERE sender_person_id = p.id) AS n
                 FROM person p WHERE p.is_self = 0 ORDER BY n DESC, p.display_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok(Person { id: r.get(0)?, display_name: r.get(1)?, items: r.get(2)? }))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// The user says two rows are one person ("Tunde" on WhatsApp is "Tunde
    /// Bakare" in SMS): `other`'s aliases, identities and items move to `keep`.
    /// Merging is the user's call, never guessed (knowledge/25).
    pub(crate) fn merge_people(&mut self, keep: i64, other: i64) -> Result<(), String> {
        if keep == other {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("UPDATE OR IGNORE person_alias SET person_id = ?1 WHERE person_id = ?2", params![keep, other]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE OR IGNORE identity SET person_id = ?1 WHERE person_id = ?2", params![keep, other]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE items SET sender_person_id = ?1 WHERE sender_person_id = ?2", params![keep, other]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE facts SET subject_person_id = ?1 WHERE subject_person_id = ?2", params![keep, other]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM person WHERE id = ?1", params![other]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// What is held, per app — shown on "What Verdure knows".
    pub(crate) fn counts(&self) -> Result<Vec<AppCount>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.app, COUNT(i.id), COUNT(DISTINCT t.id)
                 FROM threads t LEFT JOIN items i ON i.thread_id = t.id
                 GROUP BY t.app ORDER BY COUNT(i.id) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok(AppCount { app: r.get(0)?, items: r.get(1)?, threads: r.get(2)? }))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Forget one conversation entirely — items, their index entries, facts.
    pub(crate) fn delete_thread(&mut self, thread_id: i64) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM items_fts WHERE rowid IN (SELECT id FROM items WHERE thread_id = ?1)",
            params![thread_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM threads WHERE id = ?1", params![thread_id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Forget one person: their items (and index entries) and the person row.
    pub(crate) fn delete_person(&mut self, person_id: i64) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM items_fts WHERE rowid IN (SELECT id FROM items WHERE sender_person_id = ?1)",
            params![person_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM items WHERE sender_person_id = ?1", params![person_id]).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM person WHERE id = ?1", params![person_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    /// Forget everything. The user's one-tap exit from the whole feature.
    pub(crate) fn delete_all(&mut self) -> Result<(), String> {
        self.conn
            .execute_batch("DELETE FROM items_fts; DELETE FROM threads; DELETE FROM person; DELETE FROM sources;")
            .map_err(|e| e.to_string())
    }
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

fn row_to_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<Item> {
    let direction: String = r.get(6)?;
    Ok(Item {
        id: r.get(0)?,
        thread_id: r.get(1)?,
        app: r.get(2)?,
        thread_title: r.get(3)?,
        sender: r.get(4)?,
        sender_person_id: r.get(5)?,
        direction: Direction::parse(&direction).as_str(),
        at_ms: r.get(7)?,
        text: r.get(8)?,
    })
}

/// A stable identity for one message, so a re-posted notification or a
/// re-imported export never stores it twice. FNV-1a rather than `DefaultHasher`
/// because the latter is not promised stable across Rust versions, and this key
/// lives in a file on a phone for years.
pub(crate) fn dedupe_key(app: &str, thread_key: &str, sender_key: &str, at_ms: i64, text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for chunk in [app.as_bytes(), b"\0", thread_key.as_bytes(), b"\0", sender_key.as_bytes(), b"\0", at_ms.to_string().as_bytes(), b"\0", text.as_bytes()] {
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

/// Turn a question into an FTS5 query: its content words, each a prefix
/// match, OR-ed. Prefixes let "rent" find "rental" and "Tunde" find "Tunde's".
/// Nothing a user can type reaches FTS5 unquoted, so its syntax can't break.
pub(crate) fn fts_query(question: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for raw in question.split(|c: char| !c.is_alphanumeric() && c != '\'') {
        let w = raw.trim_matches('\'').to_lowercase();
        if w.len() < 2 || STOPWORDS.contains(&w.as_str()) || words.contains(&w) {
            continue;
        }
        words.push(w);
    }
    words
        .iter()
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

/// Stored items as the prompt's sources (knowledge/24's block, reused): the
/// sender and thread stand where a page title stood, the app where the domain
/// did, and the "url" is Verdure's own thread link for the footer to open.
pub(crate) fn items_as_snippets(items: &[Item]) -> Vec<Snippet> {
    items
        .iter()
        .map(|i| Snippet {
            title: if i.direction == "out" {
                format!("You, in {}", i.thread_title)
            } else if i.thread_title == i.sender || i.thread_title.is_empty() {
                i.sender.clone()
            } else {
                format!("{} in {}", i.sender, i.thread_title)
            },
            url: format!("verdure://thread/{}", i.thread_id),
            domain: app_label(&i.app).to_string(),
            date: Some(format_date(i.at_ms)),
            text: i.text.clone(),
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

/// Parse a WhatsApp "Export chat" text file into items — the one-time,
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
pub(crate) fn parse_whatsapp_export(text: &str, thread_title: &str, me: &str) -> Vec<NewItem> {
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
    let mut items: Vec<NewItem> = Vec::new();
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
        items.push(NewItem {
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

    fn msg(app: &str, thread: &str, sender: &str, at_ms: i64, text: &str) -> NewItem {
        NewItem {
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
    }

    #[test]
    fn ingest_stores_once_and_recognises_a_repost() {
        let mut s = Store::open_in_memory().unwrap();
        let m = msg("com.whatsapp", "Tunde", "Tunde", 1_000, "hello");
        assert!(matches!(s.ingest(&m).unwrap(), Ingest::Inserted(_)));
        // A notification update re-posts the same message: not stored again.
        assert_eq!(s.ingest(&m).unwrap(), Ingest::Duplicate);
        // The same words at a different time ARE a new message.
        let later = NewItem { at_ms: 5_000, ..m.clone() };
        assert!(matches!(s.ingest(&later).unwrap(), Ingest::Inserted(_)));
        assert_eq!(s.counts().unwrap()[0].items, 2);
    }

    #[test]
    fn the_last_thing_someone_said_is_a_lookup_not_a_search() {
        let s = seeded();
        let tunde = one(s.find_person("Tunde").unwrap());
        let last = s.last_from(tunde.id, 1).unwrap();
        assert_eq!(last[0].text, "See you Friday then");
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
        s.merge_people(ada.id, full.id).unwrap();
        assert_eq!(s.people().unwrap().len(), 1);
        assert_eq!(s.last_from(ada.id, 10).unwrap().len(), 3);
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
        s.delete_all().unwrap();
        assert!(s.threads(10).unwrap().is_empty());
        assert!(s.people().unwrap().is_empty());
        assert!(s.counts().unwrap().is_empty());
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
    fn items_become_sources_the_prompt_already_understands() {
        let s = seeded();
        let hits = s.search("rent", None, 5).unwrap();
        let snippets = items_as_snippets(&hits);
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
