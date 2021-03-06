use secrecy::ExposeSecret;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use domains::models::users::{HashedPassword, User, UserId, UserName};
use domains::models::EmailAddress;

#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    /// 予期していないエラー
    #[error(transparent)]
    UnexpectedError(anyhow::Error),
    /// ドメイン制約エラー
    #[error("{0}")]
    DomainError(anyhow::Error),
    /// ユーザー登録エラー
    #[error("ユーザーを登録できませんでした。")]
    CreateError,
    /// ユーザー存在エラー
    #[error("ユーザー({0})が存在しません。")]
    NotFoundError(Uuid),
}

#[derive(Default)]
pub struct PgUserRepository;

impl PgUserRepository {
    /// Eメールアドレスからユーザーを取得する。
    ///
    /// # Argument:
    ///
    /// * `email_address` - Eメールアドレス。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// ユーザーインスタンス。ユーザーが見つからなかった場合は`None`。
    pub async fn get_by_email_address(
        &self,
        email_address: &EmailAddress,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<User>, UserRepositoryError> {
        // データーベースに問い合わせ
        let result = sqlx::query!(
            r#"
            SELECT
                id, user_name, email_address, hashed_password, is_active,
                last_logged_in, created_at, updated_at
            FROM
                users
            WHERE
                email_address = $1
            "#,
            email_address.value()
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // ユーザーを取得できなかった場合、Noneを返却
        if result.is_none() {
            return Ok(None);
        }
        let record = result.unwrap();
        let id = UserId::new(record.id);
        let user_name =
            UserName::new(&record.user_name).map_err(UserRepositoryError::DomainError)?;
        let hashed_password = HashedPassword::new_unchecked(&record.hashed_password);
        let user = User::new(
            id,
            user_name,
            (*email_address).clone(),
            hashed_password,
            record.is_active,
            record.last_logged_in,
            Some(record.created_at),
            Some(record.updated_at),
        );

        Ok(Some(user))
    }

    /// ユーザーを取得する。
    ///
    /// # Arguments
    ///
    /// * `id` - 取得するユーザーのユーザーID。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// ユーザーインスタンス。ユーザーが見つからなかった場合は`None`。
    pub async fn get_by_id(
        &self,
        id: UserId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<User>, UserRepositoryError> {
        // データーベースに問い合わせ
        let result = sqlx::query!(
            r#"
            SELECT
                user_name, email_address, hashed_password, is_active,
                last_logged_in, created_at, updated_at
            FROM
                users
            WHERE
                id = $1
            "#,
            id.value()
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // ユーザーを取得できなかった場合、エラーを返却
        if result.is_none() {
            return Ok(None);
        }
        // ユーザーを取得
        let record = result.unwrap();
        let user_name =
            UserName::new(&record.user_name).map_err(UserRepositoryError::DomainError)?;
        let email_address =
            EmailAddress::new(&record.email_address).map_err(UserRepositoryError::DomainError)?;
        let hashed_password = HashedPassword::new_unchecked(&record.hashed_password);
        let user = User::new(
            id.clone(),
            user_name,
            email_address,
            hashed_password,
            record.is_active,
            record.last_logged_in,
            Some(record.created_at),
            Some(record.updated_at),
        );

        Ok(Some(user))
    }

    /// ユーザーを登録する。
    ///
    /// # Arguments
    ///
    /// * `user` - 登録するユーザーのユーザーインスタンス。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// 登録したユーザーのユーザーインスタンス。
    pub async fn insert(
        &self,
        user: &User,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<User, UserRepositoryError> {
        // ユーザーを登録
        let result = sqlx::query!(
            r#"
            INSERT INTO users (
                id, user_name, email_address, hashed_password,
                is_active, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, current_timestamp, current_timestamp
            )
            "#,
            user.id().value(),
            user.user_name().value(),
            user.email_address().value(),
            user.hashed_password().value().expose_secret(),
            user.is_active(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // ユーザーが登録されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::CreateError);
        }
        // 作成日時と更新日時を取得するため、登録したユーザーを取得
        let inserted_user = self.get_by_id(user.id(), &mut *tx).await?;
        if inserted_user.is_none() {
            return Err(UserRepositoryError::CreateError);
        }

        Ok(inserted_user.unwrap())
    }

    /// ユーザーを更新する。
    ///
    /// ユーザー名、アクティブフラグ及び更新日時を更新する。
    ///
    /// # Arguments
    ///
    /// * `user` - 更新するユーザーのユーザーインスタンス。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// 更新したユーザーの更新後のユーザーインスタンス。
    pub async fn update(
        &self,
        user: &User,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<User, UserRepositoryError> {
        // データベースを操作
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET
                user_name = $1,
                is_active = $2,
                updated_at = current_timestamp
            WHERE
                id = $3
            "#,
            user.user_name().value(),
            user.is_active(),
            user.id().value(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // ユーザーが更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::NotFoundError(user.id().value()));
        }
        // 更新日時を取得するため、登録したユーザーを取得
        let updated_user = self.get_by_id(user.id(), &mut *tx).await?;

        Ok(updated_user.unwrap())
    }

    /// ユーザーを削除する。
    ///
    /// # Arguments
    ///
    /// * `id` - 削除するユーザーのID。
    /// * `tx` - トランザクション。
    pub async fn delete(
        &self,
        id: UserId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), UserRepositoryError> {
        // データベースを操作
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE
                id = $1
            "#,
            id.value()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // ユーザーが削除されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::NotFoundError(id.value()));
        }

        Ok(())
    }

    /// パスワードを変更する。
    ///
    /// # Arguments
    ///
    /// * `id` - パスワードを変更するユーザーのID。
    /// * `hashed_password` - 新たに設定するハッシュ化したパスワード。
    pub async fn change_password(
        &self,
        id: UserId,
        hashed_password: HashedPassword,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), UserRepositoryError> {
        // データベースを操作
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET
                hashed_password = $1,
                updated_at = current_timestamp
            WHERE
                id = $2
            "#,
            hashed_password.value().expose_secret(),
            id.value(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // パスワードが更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::NotFoundError(id.value()));
        }

        Ok(())
    }

    /// 最終ログイン日時に現在日時を設定する。
    ///
    /// # Arguments
    ///
    /// * `id` - 最終ログイン日時を設定するユーザーのID。
    /// * `tx` - トランサクジョン。
    pub async fn update_last_logged_in(
        &self,
        id: UserId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), UserRepositoryError> {
        // データベースを操作
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET
                last_logged_in = current_timestamp,
                updated_at = current_timestamp
            WHERE
                id = $1
            "#,
            id.value(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::UnexpectedError(e.into()))?;
        // 最終ログイン日時が更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::NotFoundError(id.value()));
        }

        Ok(())
    }
}
