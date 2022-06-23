use anyhow::anyhow;
use secrecy::ExposeSecret;
use sqlx::{Postgres, Transaction};

use domains::models::base::EmailAddress;
use domains::models::users::{HashedPassword, User, UserId, UserName};

pub struct PgUserRepository;

impl PgUserRepository {
    pub async fn get(
        &self,
        id: &UserId,
        tx: &mut Transaction<'_, Postgres>,
    ) -> anyhow::Result<User> {
        let result = sqlx::query!(
            r#"
            SELECT
                user_name, email_address, hashed_password, last_logged_in,
                created_at, updated_at
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
        let user = User::gen(
            id.clone(),
            UserName::gen(&result.user_name)?,
            EmailAddress::gen(&result.email_address)?,
            HashedPassword::gen_unchecked(&result.hashed_password),
            result.last_logged_in,
            Some(result.created_at),
            Some(result.updated_at),
        );

        Ok(user)
    }

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
                created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, current_timestamp, current_timestamp
            )"#,
            user.id().value(),
            user.user_name().value(),
            user.email_address().value(),
            user.hashed_password().value().expose_secret()
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
}
