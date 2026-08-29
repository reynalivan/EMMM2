pub mod grouping;
pub mod hashing;
pub mod resolver;
pub mod scanner;
pub mod signals;
pub mod snapshot;

/// Similarity of two byte sizes, in `0.0..=1.0`.
///
/// Two zero sizes are treated as a perfect match — equal is equal. Both call
/// sites reject empty file lists before scoring, so that case is not reachable
/// in practice; it is defined here so the two passes cannot disagree about it,
/// which they previously did (0.0 in the prefilter, 1.0 in the scorer).
pub(super) fn size_ratio(left: u64, right: u64) -> f64 {
    let max = left.max(right);
    if max == 0 {
        return 1.0;
    }
    left.min(right) as f64 / max as f64
}
