//! All functions and structures for `Comparator` struct allowing
//! easy algorithms comparison.
pub(crate) mod compare;
pub(crate) mod stats;

/// Convert nano seconds to human readable string.
pub(crate) fn time_string(nano: u64) -> String {
    match nano {
        n if n < 1_000 => format!("{}ns", n),
        n if n < 1_000_000 => format!("{:.2}us", (n as f64 / 1_000.0)),
        n if n < 1_000_000_000 => format!("{:.2}ms", (n as f64 / 1_000_000.0)),
        n if n < 60_000_000_000 => format!("{:.2}s", (n as f64 / 1_000_000_000.0)),
        n => format!("{}m{}s", n / 60_000_000_000, n % 60_000_000_000),
    }
}
