//! LongMemEval Benchmark for arawn-memory
//!
//! Evaluates our vector search (sqlite-vec + all-MiniLM-L6-v2) against the
//! LongMemEval dataset — the same external benchmark used by MemPalace.
//! Provides a comparable, reproducible vector-retrieval quality baseline
//! against a public corpus.
//!
//! **What this measures**: per-question retrieval quality. For each question,
//! arawn embeds the ~50-session haystack, embeds the query, ranks by cosine
//! similarity, and computes Recall@5 / Recall@10 / NDCG@10 against the
//! provided ground-truth session ids. Aggregated across all 500 questions.
//!
//! **What this does NOT measure**: anything arawn-specific — lens
//! scoping, ontology-tagged retrieval, FTS interplay, MemoryStack L1/L2, or
//! the steward's role in pruning stale entities. For those, see
//! `recall_eval.rs` (sibling file, runs by default with arawn-shaped
//! fixtures).
//!
//! **Dataset**: LongMemEval — 500 questions across ~19K conversation
//! sessions, downloaded on first run. Cached under
//! `arawn-memory/tests/fixtures/longmemeval/` once fetched.
//!
//! **Cost**: requires the all-MiniLM-L6-v2 ONNX model (~90 MB) and
//! ~5 minutes wall-clock. Ignored by default.
//!
//! Run with:
//!
//! ```text
//! cargo test -p arawn-memory --test longmemeval_bench -- --nocapture --ignored
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use arawn_memory::*;

/// Temporal proximity score: higher for sessions closer in time to the question.
// ============================================================================
// Dataset types
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct LongMemEvalEntry {
    question: String,
    #[serde(default)]
    question_type: Option<String>,
    haystack_sessions: Vec<Vec<Turn>>,
    haystack_session_ids: Vec<String>,
    #[serde(default)]
    ground_truth_session_ids: Vec<String>,
    // Some versions use "answer_session_ids" instead
    #[serde(default)]
    answer_session_ids: Vec<String>,
}

impl LongMemEvalEntry {
    fn ground_truth_ids(&self) -> &[String] {
        if !self.ground_truth_session_ids.is_empty() {
            &self.ground_truth_session_ids
        } else {
            &self.answer_session_ids
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct Turn {
    role: String,
    content: String,
}

// ============================================================================
// Metrics
// ============================================================================

/// Recall@K (any): at least one ground-truth session appears in top-K.
fn recall_any_at_k(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64 {
    let top_k: Vec<&str> = retrieved_ids.iter().take(k).copied().collect();
    let hit = ground_truth_ids
        .iter()
        .any(|gt| top_k.contains(&gt.as_str()));
    if hit { 1.0 } else { 0.0 }
}

/// Recall@K (all): all ground-truth sessions appear in top-K.
fn recall_all_at_k(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64 {
    let top_k: std::collections::HashSet<&str> = retrieved_ids.iter().take(k).copied().collect();
    let all_found = ground_truth_ids
        .iter()
        .all(|gt| top_k.contains(gt.as_str()));
    if all_found { 1.0 } else { 0.0 }
}

/// NDCG@K: Normalized Discounted Cumulative Gain.
fn ndcg_at_k(retrieved_ids: &[&str], ground_truth_ids: &[String], k: usize) -> f64 {
    let gt_set: std::collections::HashSet<&str> =
        ground_truth_ids.iter().map(|s| s.as_str()).collect();

    // DCG
    let mut dcg = 0.0f64;
    for (i, id) in retrieved_ids.iter().take(k).enumerate() {
        if gt_set.contains(*id) {
            dcg += 1.0 / (i as f64 + 2.0).log2();
        }
    }

    // Ideal DCG (all relevant at top)
    let num_relevant = ground_truth_ids.len().min(k);
    let mut idcg = 0.0f64;
    for i in 0..num_relevant {
        idcg += 1.0 / (i as f64 + 2.0).log2();
    }

    if idcg == 0.0 { 0.0 } else { dcg / idcg }
}

// ============================================================================
// Dataset download
// ============================================================================

const DATASET_URL: &str = "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/longmemeval_s_cleaned.json";

fn dataset_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".arawn/benchmarks/longmemeval_s_cleaned.json")
}

fn download_dataset() -> Result<PathBuf, String> {
    let path = dataset_path();
    if path.exists() {
        return Ok(path);
    }

    println!("  Downloading LongMemEval dataset from HuggingFace...");
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| format!("create dir: {e}"))?;

    let response = ureq::get(DATASET_URL)
        .call()
        .map_err(|e| format!("download: {e}"))?;

    let mut reader = response.into_body().into_reader();
    let mut file = std::fs::File::create(&path).map_err(|e| format!("create file: {e}"))?;
    std::io::copy(&mut reader, &mut file).map_err(|e| format!("write file: {e}"))?;

    println!("  Dataset saved to {}", path.display());
    Ok(path)
}

fn load_dataset(path: &PathBuf) -> Vec<LongMemEvalEntry> {
    let data = std::fs::read_to_string(path).expect("read dataset");
    serde_json::from_str(&data).expect("parse dataset JSON")
}

// ============================================================================
// Benchmark
// ============================================================================

#[test]
#[ignore] // Run explicitly: cargo test -p arawn-memory --test longmemeval_bench -- --ignored --nocapture
fn longmemeval_benchmark() {
    println!("\n======================================================================");
    println!("  LongMemEval Benchmark — arawn-memory vector search");
    println!("  Model: all-MiniLM-L6-v2 (384 dims, local ONNX)");
    println!("======================================================================\n");

    // Download/load dataset
    let dataset_path = match download_dataset() {
        Ok(p) => p,
        Err(e) => {
            println!("  SKIPPING: Could not load dataset: {e}");
            return;
        }
    };

    let entries = load_dataset(&dataset_path);
    println!("  Loaded {} questions", entries.len());

    // Create embedder
    let config = arawn_embed::EmbeddingConfig::default();
    let embedder = match arawn_embed::create_embedder(&config) {
        Ok(e) => e,
        Err(e) => {
            println!("  SKIPPING: Embedder unavailable: {e}");
            return;
        }
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Per-question indexing (matches MemPalace methodology):
    // Each question has its own haystack of ~50 sessions.
    // Build a fresh vector store per question, embed user turns only.
    arawn_memory::init_vector_extension();

    // Pre-compute embeddings for all unique session user-turn documents.
    // This avoids re-embedding the same session across multiple questions.
    println!("  Pre-computing embeddings for unique sessions...");

    // Collect unique session docs (user turns only, matching MemPalace raw mode)
    let mut session_docs: HashMap<String, String> = HashMap::new();
    for entry in &entries {
        for (i, session) in entry.haystack_sessions.iter().enumerate() {
            let session_id = &entry.haystack_session_ids[i];
            if session_docs.contains_key(session_id) {
                continue;
            }
            let user_turns: Vec<&str> = session
                .iter()
                .filter(|t| t.role == "user")
                .map(|t| t.content.as_str())
                .collect();
            if !user_turns.is_empty() {
                session_docs.insert(session_id.clone(), user_turns.join("\n"));
            }
        }
    }

    println!("  {} unique session documents to embed", session_docs.len());

    // Embed all session docs in batches
    let doc_list: Vec<(&String, &String)> = session_docs.iter().collect();
    let mut session_embeddings: HashMap<String, Vec<f32>> = HashMap::new();
    let batch_size = 32;
    let total_batches = (doc_list.len() + batch_size - 1) / batch_size;

    for (batch_idx, chunk) in doc_list.chunks(batch_size).enumerate() {
        if (batch_idx + 1) % 100 == 0 || batch_idx == 0 {
            println!(
                "    Batch {}/{} ({} sessions embedded)",
                batch_idx + 1,
                total_batches,
                batch_idx * batch_size
            );
        }

        let texts: Vec<&str> = chunk.iter().map(|(_, doc)| doc.as_str()).collect();
        let embeddings = rt.block_on(embedder.embed_batch(&texts)).unwrap();

        for (i, (session_id, _)) in chunk.iter().enumerate() {
            session_embeddings.insert((*session_id).clone(), embeddings[i].clone());
        }
    }

    println!(
        "  Embeddings cached for {} sessions",
        session_embeddings.len()
    );
    println!(
        "\n  Evaluating {} questions (per-question index, user-turns only)...\n",
        entries.len()
    );

    // Evaluate each question with its own haystack
    let mut results_by_type: HashMap<String, Vec<(f64, f64, f64, f64)>> = HashMap::new();
    let mut total_r5_any = 0.0;
    let mut total_r5_all = 0.0;
    let mut total_r10_any = 0.0;
    let mut total_ndcg10 = 0.0;
    let mut count = 0;

    for (qi, entry) in entries.iter().enumerate() {
        let ground_truth = entry.ground_truth_ids();
        if ground_truth.is_empty() {
            continue;
        }

        // Build per-question vector index from this question's haystack
        let store = MemoryStore::in_memory().unwrap();
        store.init_vectors(384).unwrap();

        let mut uuid_to_session: HashMap<uuid::Uuid, String> = HashMap::new();
        let mut indexed = 0;

        for session_id in entry.haystack_session_ids.iter() {
            if let Some(emb) = session_embeddings.get(session_id) {
                let entity_uuid = uuid::Uuid::new_v4();
                store.store_embedding(entity_uuid, emb).unwrap();
                uuid_to_session.insert(entity_uuid, session_id.clone());
                indexed += 1;
            }
        }

        if indexed == 0 {
            continue;
        }

        // Embed the question and search
        let q_emb = rt.block_on(embedder.embed(&entry.question)).unwrap();
        let results = store.search_similar(&q_emb, 50.min(indexed)).unwrap();

        // Deduplicate to unique session IDs
        let mut seen: HashSet<String> = HashSet::new();
        let mut retrieved_session_ids: Vec<String> = Vec::new();
        for r in &results {
            if let Some(sid) = uuid_to_session.get(&r.entity_id) {
                if seen.insert(sid.clone()) {
                    retrieved_session_ids.push(sid.clone());
                }
            }
        }

        let retrieved_refs: Vec<&str> = retrieved_session_ids.iter().map(|s| s.as_str()).collect();

        let r5_any = recall_any_at_k(&retrieved_refs, ground_truth, 5);
        let r5_all = recall_all_at_k(&retrieved_refs, ground_truth, 5);
        let r10_any = recall_any_at_k(&retrieved_refs, ground_truth, 10);
        let ndcg = ndcg_at_k(&retrieved_refs, ground_truth, 10);

        total_r5_any += r5_any;
        total_r5_all += r5_all;
        total_r10_any += r10_any;
        total_ndcg10 += ndcg;
        count += 1;

        let qtype = entry
            .question_type
            .as_deref()
            .unwrap_or("unknown")
            .to_string();
        results_by_type
            .entry(qtype)
            .or_default()
            .push((r5_any, r5_all, r10_any, ndcg));

        if (qi + 1) % 100 == 0 {
            let running_r5 = total_r5_any / count as f64 * 100.0;
            println!(
                "    [{}/{}] running R@5(any): {:.1}%",
                qi + 1,
                entries.len(),
                running_r5
            );
        }
    }

    // Print results
    println!("\n----------------------------------------------------------------------");
    println!(
        "  {:<35} {:>8} {:>8} {:>8} {:>8} {:>5}",
        "Question Type", "R@5any", "R@5all", "R@10any", "NDCG@10", "N"
    );
    println!("----------------------------------------------------------------------");

    let mut types: Vec<(&String, &Vec<(f64, f64, f64, f64)>)> = results_by_type.iter().collect();
    types.sort_by_key(|(t, _)| t.to_string());

    for (qtype, scores) in &types {
        let n = scores.len() as f64;
        let avg_r5_any: f64 = scores.iter().map(|(r, _, _, _)| r).sum::<f64>() / n;
        let avg_r5_all: f64 = scores.iter().map(|(_, r, _, _)| r).sum::<f64>() / n;
        let avg_r10_any: f64 = scores.iter().map(|(_, _, r, _)| r).sum::<f64>() / n;
        let avg_ndcg: f64 = scores.iter().map(|(_, _, _, n)| n).sum::<f64>() / n;
        println!(
            "  {:<35} {:>7.1}% {:>7.1}% {:>7.1}% {:>8.3} {:>5}",
            qtype,
            avg_r5_any * 100.0,
            avg_r5_all * 100.0,
            avg_r10_any * 100.0,
            avg_ndcg,
            scores.len()
        );
    }

    let avg_r5_any = total_r5_any / count as f64;
    let avg_r5_all = total_r5_all / count as f64;
    let avg_r10_any = total_r10_any / count as f64;
    let avg_ndcg = total_ndcg10 / count as f64;

    println!("----------------------------------------------------------------------");
    println!(
        "  {:<35} {:>7.1}% {:>7.1}% {:>7.1}% {:>8.3} {:>5}",
        "OVERALL",
        avg_r5_any * 100.0,
        avg_r5_all * 100.0,
        avg_r10_any * 100.0,
        avg_ndcg,
        count
    );
    println!("======================================================================");
    println!("\n  MemPalace baseline (raw mode, same model):        R@5 = 96.6%");
    println!("  v1 (global index, session-level):               R@5 = 28.4%");
    println!("  v2 (global index, turn-level):                  R@5 = 38.8%");
    println!(
        "  v5 (per-question index, user-turns, session):   R@5 = {:.1}%\n",
        avg_r5_any * 100.0
    );

    // Don't assert a threshold — this is a benchmark, not a pass/fail test.
    // Just report the numbers for comparison.
}
