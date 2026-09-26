use crate::domain::DomainError;

/// Cosine similarity between two equal-length vectors, clamped to `[-1, 1]`.
///
/// Shared by every `VectorStore` adapter so scoring stays identical across the
/// Sled and SQLite stores.
pub(crate) fn cosine_similarity(query: &[f32], candidate: &[f32]) -> Result<f32, DomainError> {
    if query.len() != candidate.len() {
        return Err(DomainError::embedding(format!(
            "embedding dimension mismatch: query {} vs candidate {}",
            query.len(),
            candidate.len()
        )));
    }

    let mut dot = 0.0f32;
    let mut q_norm = 0.0f32;
    let mut c_norm = 0.0f32;

    for (q, c) in query.iter().zip(candidate.iter()) {
        dot += q * c;
        q_norm += q * q;
        c_norm += c * c;
    }

    let denom = q_norm.sqrt() * c_norm.sqrt();
    if denom == 0.0 {
        return Err(DomainError::embedding(
            "cannot compute cosine similarity with zero vector",
        ));
    }

    Ok((dot / denom).clamp(-1.0, 1.0))
}
