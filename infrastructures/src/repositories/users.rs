use secrecy::ExposeSecret;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use domains::models::base::EmailAddress;
use domains::models::users::{HashedPassword, User, UserId, UserName};

#[derive(Debug, thiserror::Error)]
pub enum UserRepositoryError {
    /// データベースエラー
    #[error("データベースがエラーを返却しました。: {0}")]
    DatabaseError(String),
    /// ユーザーが見つからない
    #[error("ユーザー({0})が存在しません。")]
    UserNotFound(Uuid),
    /// ドメイン制約エラー
    #[error("{0}")]
    DomainRestrictionError(#[from] anyhow::Error),
    /// ユーザー登録エラー
    #[error("ユーザーを登録できませんでした。")]
    UserCreateError,
}

pub struct PgUserRepository;

impl PgUserRepository {
    /// ユーザーを取得する。
    ///
    /// # Arguments
    ///
    /// * `id` - 取得するユーザーのユーザーID。
    /// * `tx` - トランザクション。
    ///
    /// # Returns
    ///
    /// ユーザーIDが一致するユーザーのユーザーインスタンス。
    pub async fn get(
        &self,
        id: &UserId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<User, UserRepositoryError> {
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
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // ユーザーを取得できなかった場合、エラーを返却
        let record = result.ok_or_else(|| UserRepositoryError::UserNotFound(*id.value()))?;
        // ユーザーを取得
        let user_name = UserName::new(&record.user_name)
            .map_err(UserRepositoryError::DomainRestrictionError)?;
        let email_address = EmailAddress::new(&record.email_address)
            .map_err(UserRepositoryError::DomainRestrictionError)?;
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

        Ok(user)
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
        // データベースを操作
        let result = sqlx::query!(
            r#"
            INSERT INTO users (
                id, user_name, email_address, hashed_password,
                is_active, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, current_timestamp, current_timestamp
            )"#,
            user.id().value(),
            user.user_name().value(),
            user.email_address().value(),
            user.hashed_password().value().expose_secret(),
            user.is_active(),
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // ユーザーが登録されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::UserCreateError);
        }
        // 作成日時と更新日時を取得するため、登録したユーザーを取得
        let inserted_user = self.get(user.id(), &mut *tx).await?;

        Ok(inserted_user)
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
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // ユーザーが更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::UserNotFound(*user.id().value()));
        }
        // 更新日時を取得するため、登録したユーザーを取得
        let updated_user = self.get(user.id(), &mut *tx).await?;

        Ok(updated_user)
    }

    /// ユーザーを削除する。
    ///
    /// # Arguments
    ///
    /// * `id` - 削除するユーザーのID。
    pub async fn delete(
        &self,
        id: &UserId,
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
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // ユーザーが削除されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::UserNotFound(*id.value()));
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
        id: &UserId,
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
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // パスワードが更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::UserNotFound(*id.value()));
        }

        Ok(())
    }

    /// 最終ログイン日時に現在日時を設定する。
    ///
    /// # Arguments
    ///
    /// * `id` - 最終ログイン日時を設定するユーザーのID。
    /// * `tx` - トランサクジョン。
    pub async fn set_last_logged_in(
        &self,
        id: &UserId,
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
        .map_err(|e| UserRepositoryError::DatabaseError(format!("{}", e)))?;
        // 最終ログイン日時が更新されたか確認
        if result.rows_affected() != 1 {
            return Err(UserRepositoryError::UserNotFound(*id.value()));
        }

        Ok(())
    }
}