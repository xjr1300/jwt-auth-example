use domains::models::{
    users::{HashedPassword, RawPassword, User, UserId, UserName},
    EmailAddress,
};

fn generate_user(user_name: &str, email_address: &str, password: &str, is_active: bool) -> User {
    let raw_password = RawPassword::new(password).unwrap();
    let hashed_password = HashedPassword::new(&raw_password).unwrap();
    User::new(
        UserId::default(),
        UserName::new(user_name).unwrap(),
        EmailAddress::new(email_address).unwrap(),
        hashed_password,
        is_active,
        None,
        None,
        None,
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

        Self {
            active_user: generate_user(
                "active-user",
                "active-user@example.com",
                &active_user_password,
                true,
            ),
            active_user_password,
            non_active_user: generate_user(
                "non-active-user",
                "non-active-user@example.com",
                &non_active_user_password,
                false,
            ),
            non_active_user_password,
        }
    }
}
