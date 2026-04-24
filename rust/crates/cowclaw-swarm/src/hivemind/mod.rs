pub mod broker;
pub mod protocol;

#[derive(Debug, thiserror::Error)]
#[error("no candidates in pool")]
pub struct NoCandidatesInWave;

pub fn pick_responder(from: &str, pool: &[String], cross_provider_pref: bool) -> Result<String, NoCandidatesInWave> {
    if pool.is_empty() {
        return Err(NoCandidatesInWave);
    }
    if cross_provider_pref {
        // Try to pick a worker from a different "provider" (prefix before first '-')
        let from_prefix = from.split('-').next().unwrap_or("");
        let other = pool.iter().find(|w| !w.starts_with(from_prefix));
        if let Some(w) = other {
            return Ok(w.clone());
        }
    }
    // Fallback: pick first that is not exactly `from`, or from itself if only one
    let candidate = pool.iter().find(|w| w.as_str() != from)
        .or_else(|| pool.first())
        .ok_or(NoCandidatesInWave)?;
    Ok(candidate.clone())
}
