use actix_web::cookie::time::OffsetDateTime;
use domains::models::{
    users::{HashedPassword, RawPassword, User, UserId, UserName},
    EmailAddress,
};
use secrecy::ExposeSecret;
use sqlx::PgPool;

fn generate_user(
    user_name: &str,
    email_address: &str,
    password: &str,
    is_active: bool,
    timestamp: OffsetDateTime,
) -> User {
    let raw_password = RawPassword::new(password).unwrap();
    let hashed_password = HashedPassword::new(&raw_password).unwrap();
    User::new(
        UserId::default(),
        UserName::new(user_name).unwrap(),
        EmailAddress::new(email_address).unwrap(),
        hashed_password,
        is_active,
        None,
        Some(timestamp),
        Some(timestamp),
    )
}

pub struct TestUsers {
    pub active_user: User,
    pub active_user_password: String,
    pub non_active_user: User,
    pub non_active_user_password: String,
}

impl TestUsers {
    pub fn default() -> Self {
        /* cSpell: disable */
        let active_user_password = "&MpHFQZKVr7i".to_owned();
        let non_active_user_password = "3nHUW@[bCs?b".to_owned();
        /* cSpell: enable */

        let timestamp = OffsetDateTime::now_utc();
        Self {
            active_user: generate_user(
                "active-user",
                "active-user@example.com",
                &active_user_password,
                true,
                timestamp,
            ),
            active_user_password,
            non_active_user: generate_user(
                "non-active-user",
                "non-active-user@example.com",
                &non_active_user_password,
                false,
                timestamp,
            ),
            non_active_user_password,
        }
    }

    /// テストユーザーをデータベースに登録する。
    pub async fn store(&self, pool: &PgPool) {
        let users: Vec<&User> = vec![&self.active_user, &self.non_active_user];
        for user in users.iter() {
            sqlx::query!(
                r#"
                INSERT INTO users (
                    id, user_name, email_address, hashed_password,
                    is_active, created_at, updated_at
                ) VALUES (
                    $1, $2, $3, $4,
                    $5, $6, $7
                )
                "#,
                user.id().value(),
                user.user_name().value(),
                user.email_address().value(),
                user.hashed_password().value().expose_secret(),
                user.is_active(),
                user.created_at().unwrap(),
                user.updated_at().unwrap(),
            )
            .execute(pool)
            .await
            .expect("テスト用のユーザーをデータベースに登録できませんでした。");
        }
    }
}
