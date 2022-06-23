use anyhow::anyhow;
use secrecy::ExposeSecret;
use sqlx::{Postgres, Transaction};

use domains::models::base::EmailAddress;
use domains::models::users::{HashedPassword, User, UserId, UserName};

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
    ) -> anyhow::Result<User> {
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
        .await?;
        if result.is_none() {
            return Err(anyhow!("ユーザーは存在しません。"));
        }
        let result = result.unwrap();
        let user = User::new(
            id.clone(),
            UserName::new(&result.user_name)?,
            EmailAddress::gen(&result.email_address)?,
            HashedPassword::gen_unchecked(&result.hashed_password),
            result.is_active,
            result.last_logged_in,
            Some(result.created_at),
            Some(result.updated_at),
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
    ) -> anyhow::Result<User> {
        // ユーザーを登録
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
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("ユーザーを登録するときにエラーが発生しました。"));
        }
        //  作成日時と更新日時を取得するため、登録したユーザーを取得
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
    ) -> anyhow::Result<User> {
        // ユーザーを更新
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
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("ユーザーが存在しません。"));
        }
        //  更新日時を取得するため、登録したユーザーを取得
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
    ) -> anyhow::Result<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE
                id = $1
            "#,
            id.value()
        )
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("ユーザーが存在しません。"));
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
    ) -> anyhow::Result<()> {
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
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("ユーザーが存在しません。"));
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
    ) -> anyhow::Result<()> {
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
        .await?;
        if result.rows_affected() != 1 {
            return Err(anyhow!("ユーザーが存在しません。"));
        }

        Ok(())
    }
}
