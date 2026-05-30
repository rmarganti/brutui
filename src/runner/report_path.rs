use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static REPORT_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn next_report_path() -> PathBuf {
    let unique = REPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!(
        "brutui-report-{}-{}-{}.json",
        std::process::id(),
        timestamp,
        unique
    ))
}
