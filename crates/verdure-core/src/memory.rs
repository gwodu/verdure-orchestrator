//! The one memory rule, in one place (knowledge/26, "Memory and thermal
//! policy"): a model may load only if `available ≥ size × 1.5 + 400 MB`.
//!
//! The spec proposed `× 1.3 + 300`. The app's own measurement says that is
//! too optimistic: a 1211 MB model peaked ~1 GB above its file size — a 490 MB
//! `CPU_REPACK` copy of the weights plus a ~500 MB compute buffer (knowledge/03,
//! 2026-09-10) — and its gate for the 1.4 GB 4B is 2300 MB. `× 1.5 + 400`
//! covers the measured 2.2 GB and stays above the app's gate; the spec's
//! rule would have admitted that model with ~300 MB to spare and let ggml
//! abort the host app. Measured beats proposed (knowledge/06, honest numbers).

/// Resident memory a model of `size_mb` on disk needs before it may load.
pub fn required_mb(size_mb: u64) -> u64 {
    size_mb * 15 / 10 + 400
}

/// Would a model of `size_mb` load into `available_mb`?
pub fn fits(size_mb: u64, available_mb: u64) -> bool {
    required_mb(size_mb) <= available_mb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rule_is_size_times_1_5_plus_400() {
        assert_eq!(required_mb(1000), 1900);
        assert_eq!(required_mb(0), 400);
    }

    #[test]
    fn the_rule_covers_what_the_app_measured() {
        // 2026-09-10: 1211 MB weights + 490 MB CPU_REPACK + ~500 MB compute.
        assert!(
            required_mb(1211) >= 1211 + 490 + 500,
            "{}",
            required_mb(1211)
        );
        // The app's gate for the 1.4 GB ByteShape 4B is 2300 MB; never below it.
        assert!(required_mb(1400) >= 2300);
        // The spec's proposal would have admitted the 1211 MB model into less
        // than it was measured to need — the reason the rule is not 1.3 + 300.
        assert!(1211 * 13 / 10 + 300 < 1211 + 490 + 500);
    }

    #[test]
    fn fits_is_the_same_rule_as_a_predicate() {
        assert!(fits(1000, 1900));
        assert!(!fits(1000, 1899));
    }
}
