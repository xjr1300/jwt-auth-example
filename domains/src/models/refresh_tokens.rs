use time::OffsetDateTime;

/// リフレッシュトークン構造体
pub struct RefreshToken {
    /// セッションID
    pub session_id: String,
    /// リフレッシュトークン
    pub refresh_token: String,
    /// 有効期限
    pub expired_at: OffsetDateTime,
}
