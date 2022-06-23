use async_trait::async_trait;

use crate::models::users::User;

#[async_trait]
pub trait UserRepository {
    async fn insert(&self, user: &User) -> anyhow::Result<User>;
}
