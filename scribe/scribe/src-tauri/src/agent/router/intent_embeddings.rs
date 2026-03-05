// Intent taxonomy and prototypes for embedding-based routing.
// Prototypes are loaded from a JSON file (see intent_prototypes.json); centroids are built once from those prototypes and cached.

use std::collections::HashMap;
use std::path::Path;

/// Intents that map 1-to-1 to direct commands (parse_goal + direct_command_to_execution).
/// Only intents for which direct_command_to_execution returns Ok are listed.
/// list_files is excluded until filesystem.list is implemented.
pub fn fast_path_intents() -> &'static [&'static str] {
    &["file_read", "file_write", "file_create", "run_command"]
}

/// Load intent prototypes from a JSON file. Format: { "intent_name": ["phrase1", "phrase2", ...], ... }.
pub fn load_prototypes_from_file(path: &Path) -> Result<HashMap<String, Vec<String>>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Read prototypes file {}: {}", path.display(), e))?;
    let raw: HashMap<String, Vec<String>> =
        serde_json::from_slice(&bytes).map_err(|e| format!("Parse prototypes JSON: {}", e))?;
    if raw.is_empty() {
        return Err("Prototypes file has no intents".to_string());
    }
    Ok(raw)
}

/// Build centroid per intent from prototype embeddings. Prototypes come from file (or caller).
/// Embeddings are computed once; dimension is inferred from the first result.
pub fn build_centroids_from_prototypes<F>(
    prototypes: &HashMap<String, Vec<String>>,
    embed_fn: &F,
) -> Result<HashMap<String, Vec<f32>>, String>
where
    F: Fn(&str) -> Result<Vec<f32>, String>,
{
    let mut centroids: HashMap<String, Vec<f32>> = HashMap::new();
    let mut dim: Option<usize> = None;

    for (intent, protos) in prototypes {
        if protos.is_empty() {
            return Err(format!("Intent {} has no prototypes", intent));
        }
        let mut sum: Option<Vec<f32>> = None;
        let mut count = 0usize;
        for p in protos {
            let v = embed_fn(p).map_err(|e| format!("Embed failed for {:?}: {}", p, e))?;
            let d = v.len();
            if let Some(ref expected) = dim {
                if d != *expected {
                    return Err(format!("Expected dim {} got {}", expected, d));
                }
            } else {
                dim = Some(d);
            }
            if sum.is_none() {
                sum = Some(vec![0.0_f32; d]);
            }
            let sum_ref = sum.as_mut().unwrap();
            for (i, x) in v.iter().enumerate() {
                sum_ref[i] += x;
            }
            count += 1;
        }
        let centroid: Vec<f32> = sum
            .unwrap()
            .iter()
            .map(|s| s / count as f32)
            .collect();
        centroids.insert(intent.clone(), centroid);
    }
    Ok(centroids)
}

/// Cosine similarity between two unit-length vectors (or any vectors; not normalized).
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-9 || norm_b < 1e-9 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_same_is_one() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal_is_zero() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }
}
