//! Local retrieval over corpora that already live on the device (knowledge/26,
//! Part 3 §3): Kolibri channels, downloaded lessons, CHW guideline text, the
//! clinician's own prior notes. Never a server-side corpus.
//!
//! One SQLite file per vendor app: chunks with their text and an optional
//! embedding, and an FTS5 index for BM25. Retrieval is **hybrid**: BM25 and
//! cosine over embeddings, fused by reciprocal rank, so an exact term and a
//! paraphrase both find their chunk. Embeddings are brute-force cosine over
//! the collection — device corpora are thousands of chunks, not millions;
//! `sqlite-vec` is the step up when a corpus outgrows that (`[OQ-111]`).
//!
//! The embedder is a port (`verdure_core::Embedder`); without one the store
//! is BM25-only and says so.

use std::path::Path;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension};
use verdure_core::{Embedder, VError};

/// A document the vendor hands in. `id` is theirs; indexing the same id again
/// replaces the old chunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    pub id: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub doc_id: String,
    pub title: String,
    pub chunk_index: u32,
    pub text: String,
    /// Fused rank score; higher is better. Comparable within one query only.
    pub score: f32,
}

pub struct Store {
    conn: Connection,
    embedder: Option<Arc<dyn Embedder>>,
}

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY,
    collection TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    title TEXT NOT NULL,
    idx INTEGER NOT NULL,
    text TEXT NOT NULL,
    embedding BLOB
);
CREATE INDEX IF NOT EXISTS chunks_coll_doc ON chunks(collection, doc_id);
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(chunk_id UNINDEXED, collection UNINDEXED, text);
";

fn db_err(e: rusqlite::Error) -> VError {
    VError::Storage(format!("index: {e}"))
}

impl Store {
    pub fn open(path: &Path, embedder: Option<Arc<dyn Embedder>>) -> Result<Store, VError> {
        let conn = Connection::open(path).map_err(db_err)?;
        conn.execute_batch(SCHEMA).map_err(db_err)?;
        Ok(Store { conn, embedder })
    }

    pub fn open_in_memory(embedder: Option<Arc<dyn Embedder>>) -> Result<Store, VError> {
        let conn = Connection::open_in_memory().map_err(db_err)?;
        conn.execute_batch(SCHEMA).map_err(db_err)?;
        Ok(Store { conn, embedder })
    }

    pub fn has_embeddings(&self) -> bool {
        self.embedder.is_some()
    }

    /// Chunk, embed and index `docs` into `collection`. Returns the number of
    /// chunks written. A doc id already in the collection is replaced.
    pub fn index(&mut self, collection: &str, docs: &[Doc]) -> Result<u64, VError> {
        let mut written = 0u64;
        let tx = self.conn.transaction().map_err(db_err)?;
        for doc in docs {
            delete_doc(&tx, collection, &doc.id)?;
            let pieces = chunk(&doc.text, 800, 120);
            let embeddings = match &self.embedder {
                Some(e) if !pieces.is_empty() => {
                    let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
                    Some(e.embed(&refs)?)
                }
                _ => None,
            };
            for (i, piece) in pieces.iter().enumerate() {
                let blob = embeddings.as_ref().map(|v| f32s_to_bytes(&v[i]));
                tx.execute(
                    "INSERT INTO chunks(collection, doc_id, title, idx, text, embedding) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![collection, doc.id, doc.title, i as i64, piece, blob],
                )
                .map_err(db_err)?;
                let id = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO chunks_fts(chunk_id, collection, text) VALUES (?1, ?2, ?3)",
                    params![id, collection, piece],
                )
                .map_err(db_err)?;
                written += 1;
            }
        }
        tx.commit().map_err(db_err)?;
        Ok(written)
    }

    /// The `k` best chunks for `question` in `collection`.
    pub fn query(&self, collection: &str, question: &str, k: u32) -> Result<Vec<Hit>, VError> {
        let k = k.max(1) as usize;
        let pool = k * 4;
        let mut ranked: Vec<Vec<i64>> = Vec::new();

        if let Some(q) = fts_query(question) {
            let mut stmt = self
                .conn
                .prepare("SELECT chunk_id FROM chunks_fts WHERE chunks_fts MATCH ?1 AND collection = ?2 ORDER BY bm25(chunks_fts) LIMIT ?3")
                .map_err(db_err)?;
            let ids: Vec<i64> = stmt
                .query_map(params![q, collection, pool as i64], |r| r.get(0))
                .map_err(db_err)?
                .collect::<Result<_, _>>()
                .map_err(db_err)?;
            ranked.push(ids);
        }

        if let Some(e) = &self.embedder {
            let qv = e.embed(&[question])?.into_iter().next().unwrap_or_default();
            let mut stmt = self
                .conn
                .prepare("SELECT id, embedding FROM chunks WHERE collection = ?1 AND embedding IS NOT NULL")
                .map_err(db_err)?;
            let mut scored: Vec<(i64, f32)> = stmt
                .query_map(params![collection], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
                })
                .map_err(db_err)?
                .flatten()
                .map(|(id, blob)| (id, cosine(&qv, &bytes_to_f32s(&blob))))
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ranked.push(scored.into_iter().take(pool).map(|(id, _)| id).collect());
        }

        let fused = reciprocal_rank_fusion(&ranked);
        let mut hits = Vec::with_capacity(k);
        for (id, score) in fused.into_iter().take(k) {
            let row = self
                .conn
                .query_row(
                    "SELECT doc_id, title, idx, text FROM chunks WHERE id = ?1",
                    params![id],
                    |r| {
                        Ok(Hit {
                            doc_id: r.get(0)?,
                            title: r.get(1)?,
                            chunk_index: r.get::<_, i64>(2)? as u32,
                            text: r.get(3)?,
                            score,
                        })
                    },
                )
                .optional()
                .map_err(db_err)?;
            if let Some(h) = row {
                hits.push(h);
            }
        }
        Ok(hits)
    }

    pub fn collections(&self) -> Result<Vec<(String, u64)>, VError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT collection, COUNT(*) FROM chunks GROUP BY collection ORDER BY collection",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
            })
            .map_err(db_err)?
            .collect::<Result<_, _>>()
            .map_err(db_err)?;
        Ok(rows)
    }

    pub fn delete_collection(&mut self, collection: &str) -> Result<(), VError> {
        let tx = self.conn.transaction().map_err(db_err)?;
        tx.execute(
            "DELETE FROM chunks_fts WHERE collection = ?1",
            params![collection],
        )
        .map_err(db_err)?;
        tx.execute(
            "DELETE FROM chunks WHERE collection = ?1",
            params![collection],
        )
        .map_err(db_err)?;
        tx.commit().map_err(db_err)
    }

    pub fn delete_all(&mut self) -> Result<(), VError> {
        self.conn
            .execute_batch("DELETE FROM chunks_fts; DELETE FROM chunks;")
            .map_err(db_err)
    }
}

fn delete_doc(
    tx: &rusqlite::Transaction<'_>,
    collection: &str,
    doc_id: &str,
) -> Result<(), VError> {
    tx.execute(
        "DELETE FROM chunks_fts WHERE chunk_id IN (SELECT id FROM chunks WHERE collection = ?1 AND doc_id = ?2)",
        params![collection, doc_id],
    )
    .map_err(db_err)?;
    tx.execute(
        "DELETE FROM chunks WHERE collection = ?1 AND doc_id = ?2",
        params![collection, doc_id],
    )
    .map_err(db_err)?;
    Ok(())
}

/// Split text into chunks of about `target` characters that end at a
/// paragraph or sentence boundary, each starting `overlap` characters before
/// the previous one ended so a fact straddling a cut is whole in one of them.
pub fn chunk(text: &str, target: usize, overlap: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= target {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let hard_end = (start + target).min(chars.len());
        let end = if hard_end == chars.len() {
            hard_end
        } else {
            // Prefer a paragraph break, then a sentence end, in the last 40 %
            // of the window; otherwise a word boundary; otherwise the hard end.
            let floor = start + target * 6 / 10;
            let window = &chars[floor..hard_end];
            let find_last = |pred: &dyn Fn(usize) -> bool| {
                window
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(i, _)| pred(*i))
                    .map(|(i, _)| floor + i + 1)
            };
            find_last(&|i| window[i] == '\n' && i > 0 && window[i - 1] == '\n')
                .or_else(|| {
                    find_last(&|i| {
                        matches!(window[i], '.' | '!' | '?')
                            && (i + 1 == window.len() || window[i + 1].is_whitespace())
                    })
                })
                .or_else(|| find_last(&|i| window[i].is_whitespace()))
                .unwrap_or(hard_end)
        };
        let piece: String = chars[start..end].iter().collect();
        let piece = piece.trim();
        if !piece.is_empty() {
            out.push(piece.to_string());
        }
        if end >= chars.len() {
            break;
        }
        start = end.saturating_sub(overlap).max(start + 1);
    }
    out
}

/// The question's content words as an FTS5 query (`"tunde"* OR "rent"*`):
/// quoted so FTS syntax in the question is neutralised, prefixed so "vaccin"
/// finds "vaccination". `None` when nothing is left to search for.
pub fn fts_query(question: &str) -> Option<String> {
    const STOP: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "of", "to", "in", "on", "at", "for", "with", "is",
        "are", "was", "were", "be", "been", "do", "does", "did", "what", "when", "where", "who",
        "whom", "which", "why", "how", "i", "me", "my", "you", "your", "it", "its", "this", "that",
        "these", "those", "about", "from", "by", "as", "did", "say", "said", "tell", "explain",
        "please",
    ];
    let words: Vec<String> = question
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().count() >= 2 && !STOP.contains(&w.as_str()))
        .map(|w| format!("\"{}\"*", w.replace('"', "")))
        .collect();
    if words.is_empty() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let uniq: Vec<String> = words
        .into_iter()
        .filter(|w| seen.insert(w.clone()))
        .collect();
    Some(uniq.join(" OR "))
}

/// Fuse ranked lists: score(id) = Σ 1 / (60 + rank). Returns ids best-first.
pub fn reciprocal_rank_fusion(lists: &[Vec<i64>]) -> Vec<(i64, f32)> {
    let mut scores: std::collections::HashMap<i64, f32> = std::collections::HashMap::new();
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            *scores.entry(*id).or_insert(0.0) += 1.0 / (60.0 + rank as f32 + 1.0);
        }
    }
    let mut out: Vec<(i64, f32)> = scores.into_iter().collect();
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    out
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

fn f32s_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_f32s(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake embedder: a bag of words hashed into 64 dimensions, with a tiny
    /// synonym table so "money for the flat" lands near "rent". Enough to
    /// show the vector leg finding what BM25 cannot.
    struct FakeEmbedder;
    impl Embedder for FakeEmbedder {
        fn id(&self) -> &str {
            "fake-64"
        }
        fn dimension(&self) -> usize {
            64
        }
        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VError> {
            Ok(texts
                .iter()
                .map(|t| {
                    let mut v = vec![0f32; 64];
                    for w in t
                        .split(|c: char| !c.is_alphanumeric())
                        .filter(|w| !w.is_empty())
                    {
                        let w = w.to_lowercase();
                        let w = match w.as_str() {
                            "money" | "flat" | "rent" | "payment" => "rent".to_string(),
                            "vaccination" | "vaccine" | "immunisation" | "jab" => {
                                "vaccine".to_string()
                            }
                            other => other.to_string(),
                        };
                        let h = w
                            .bytes()
                            .fold(7u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
                        v[(h % 64) as usize] += 1.0;
                    }
                    v
                })
                .collect())
        }
    }

    fn docs() -> Vec<Doc> {
        vec![
            Doc { id: "anc".into(), title: "Antenatal care".into(), text: "Antenatal visits: check blood pressure at every visit. Give iron and folic acid. Refer if blood pressure is above 140/90.".into() },
            Doc { id: "epi".into(), title: "Immunisation schedule".into(), text: "Immunisation at birth: BCG and OPV0. At six weeks: pentavalent 1, PCV1, OPV1, rota 1. Measles at nine months.".into() },
            Doc { id: "chat".into(), title: "Tunde".into(), text: "Tunde: I will send the rent on Friday. Also the generator needs fuel.".into() },
        ]
    }

    #[test]
    fn exact_terms_are_found_by_bm25_alone() {
        let mut s = Store::open_in_memory(None).unwrap();
        assert!(!s.has_embeddings());
        assert_eq!(s.index("guides", &docs()).unwrap(), 3);
        let hits = s
            .query("guides", "what is the measles vaccination age?", 3)
            .unwrap();
        assert_eq!(hits[0].doc_id, "epi", "{hits:?}");
        let hits = s
            .query("guides", "when do I refer for blood pressure", 2)
            .unwrap();
        assert_eq!(hits[0].doc_id, "anc");
        assert!(
            s.query("guides", "the and of", 3).unwrap().is_empty(),
            "stopwords only → nothing"
        );
        assert!(
            s.query("other", "measles", 3).unwrap().is_empty(),
            "collections are isolated"
        );
    }

    #[test]
    fn the_vector_leg_finds_a_paraphrase_bm25_misses() {
        let mut bm25 = Store::open_in_memory(None).unwrap();
        bm25.index("c", &docs()).unwrap();
        assert!(
            bm25.query("c", "money for the flat", 3).unwrap().is_empty(),
            "no shared word with 'rent'"
        );

        let mut hybrid = Store::open_in_memory(Some(Arc::new(FakeEmbedder))).unwrap();
        hybrid.index("c", &docs()).unwrap();
        let hits = hybrid.query("c", "money for the flat", 3).unwrap();
        assert_eq!(hits[0].doc_id, "chat", "{hits:?}");
        // And an exact term still wins with both legs voting.
        let hits = hybrid.query("c", "pentavalent", 3).unwrap();
        assert_eq!(hits[0].doc_id, "epi");
        assert!(hits[0].score > hits.get(1).map(|h| h.score).unwrap_or(0.0));
    }

    #[test]
    fn reindexing_a_doc_replaces_it_and_deletes_forget_the_index() {
        let mut s = Store::open_in_memory(None).unwrap();
        s.index("c", &docs()).unwrap();
        s.index(
            "c",
            &[Doc {
                id: "chat".into(),
                title: "Tunde".into(),
                text: "Tunde: paid.".into(),
            }],
        )
        .unwrap();
        assert!(
            s.query("c", "generator fuel", 3).unwrap().is_empty(),
            "the old chunk is gone"
        );
        assert_eq!(s.query("c", "paid", 3).unwrap()[0].doc_id, "chat");
        assert_eq!(s.collections().unwrap(), vec![("c".to_string(), 3)]);
        s.index("d", &docs()[..1]).unwrap();
        s.delete_collection("c").unwrap();
        assert_eq!(s.collections().unwrap(), vec![("d".to_string(), 1)]);
        assert!(s.query("c", "measles", 3).unwrap().is_empty());
        s.delete_all().unwrap();
        assert!(s.collections().unwrap().is_empty());
        assert!(
            s.query("d", "iron", 3).unwrap().is_empty(),
            "FTS rows went with the chunks"
        );
    }

    #[test]
    fn chunks_end_at_boundaries_and_overlap() {
        let para = "Sentence one is here. Sentence two follows it. ".repeat(30); // ~1400 chars
        let pieces = chunk(&para, 800, 120);
        assert!(pieces.len() >= 2, "{}", pieces.len());
        for p in &pieces {
            assert!(p.chars().count() <= 800);
            assert!(p.ends_with('.'), "cut mid-sentence: {p:?}");
        }
        // Overlap: the second chunk begins with text present at the end of the first.
        let tail: String = pieces[0]
            .chars()
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert!(pieces[1].contains(tail.trim()), "no overlap between chunks");
        assert_eq!(chunk("short", 800, 120), vec!["short"]);
        assert!(chunk("   ", 800, 120).is_empty());
        // Paragraph breaks are preferred over sentence ends.
        let two = format!("{}\n\n{}", "A".repeat(500), "B".repeat(500));
        let pieces = chunk(&two, 800, 0);
        assert_eq!(pieces[0], "A".repeat(500));
    }

    #[test]
    fn the_fts_query_neutralises_syntax_and_fusion_rewards_agreement() {
        assert_eq!(
            fts_query("What did Tunde say about the rent?").as_deref(),
            Some("\"tunde\"* OR \"rent\"*")
        );
        assert_eq!(
            fts_query("NEAR(a b) OR \"x\"").as_deref(),
            Some("\"near\"*")
        );
        assert_eq!(fts_query("the of and"), None);
        let fused = reciprocal_rank_fusion(&[vec![1, 2, 3], vec![2, 1, 4]]);
        let ids: Vec<i64> = fused.iter().map(|(id, _)| *id).collect();
        assert_eq!(&ids[..2], &[1, 2]);
        assert!(ids.contains(&3) && ids.contains(&4));
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert_eq!(
            cosine(&[1.0], &[1.0, 2.0]),
            0.0,
            "mismatched lengths score zero, never panic"
        );
    }

    #[test]
    fn a_file_backed_store_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.sqlite");
        Store::open(&path, None)
            .unwrap()
            .index("c", &docs())
            .unwrap();
        let s = Store::open(&path, None).unwrap();
        assert_eq!(s.query("c", "folic acid", 1).unwrap()[0].doc_id, "anc");
    }
}
