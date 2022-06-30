use sqlx::{Postgres, Transaction};

use domains::models::refresh_tokens::RefreshToken;

#[derive(Debug, thiserror::Error)]
pub enum RefreshTokenRepositoryError {
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    #[error("リフレッシュトークンを登録または更新できませんでした。")]
    UpsertError,
    #[error("リフレッシュトークン({0})が存在しません。")]
    NotFoundError(String),
}

#[derive(Default)]
pub struct PgRefreshTokenRepository;

impl PgRefreshTokenRepository {
    /// セッションIDからリフレッシュトークンを取得する。
    pub async fn get_by_session_id(
        session_id: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<RefreshToken>, RefreshTokenRepositoryError> {
        // データベースに問い合わせ
        let result = sqlx::query!(
            r#"
                SELECT session_id, refresh_token, expired_at
                FROM refresh_tokens
                WHERE session_id = $1
            "#,
            session_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| RefreshTokenRepositoryError::UnexpectedError(e.into()))?;
        // リフレッシュトークンを取得できなかった場合、Noneを返却
        if result.is_none() {
            return Ok(None);
        }
        let record = result.unwrap();

        Ok(Some(RefreshToken {
            session_id: record.session_id,
            token: record.refresh_token,
            expired_at: record.expired_at,
        }))
    }

    /// リフレッシュトークンを登録または更新する。
    ///
    /// セッションIDが一致するリフレッシュトークンを削除して、リフレッシュトークンを登録する。
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - 登録するリフレッシュトークン。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// ()。
    pub async fn upsert(
        &self,
        refresh_token: &RefreshToken,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), RefreshTokenRepositoryError> {
        // リフレッシュトークンを削除
        if let Err(e) = self.delete(&refresh_token.session_id, &mut *tx).await {
            match e {
                RefreshTokenRepositoryError::NotFoundError(_) => (),
                _ => {
                    return Err(e);
                }
            }
        }

        // リフレッシュトークンを登録
        let result = sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (
                session_id, refresh_token, expired_at
            ) VALUES (
                $1, $2, $3
            )
            "#,
            refresh_token.session_id,
            refresh_token.token,
            refresh_token.expired_at,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| RefreshTokenRepositoryError::UnexpectedError(e.into()))?;
        if result.rows_affected() != 1 {
            return Err(RefreshTokenRepositoryError::UpsertError);
        }

        Ok(())
    }

    /// リフレッシュトークンを削除する。
    ///
    /// # Arguments
    ///
    /// * `session_id` - リフレッシュトークンを削除するセッションID。
    /// * `tx` - トランザクション。
    pub async fn delete(
        &self,
        session_id: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), RefreshTokenRepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE session_id = $1
            "#,
            session_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| RefreshTokenRepositoryError::UnexpectedError(e.into()))?;
        if result.rows_affected() != 1 {
            return Err(RefreshTokenRepositoryError::NotFoundError(
                session_id.to_owned(),
            ));
        }

        Ok(())
    }
}
