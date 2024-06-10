use std::time::{SystemTime, UNIX_EPOCH};
pub fn unix_timestamp() -> u64 {
    let now = SystemTime::now();
    let since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Time travel is not allowed");
    since_epoch.as_secs()
}
