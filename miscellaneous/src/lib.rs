use std::time::SystemTime;

/// 現在日時をUNIXエポック秒で取得する。
///
/// # Returns
///
/// 現在日時を示すUNIXエポック秒。
pub fn current_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
