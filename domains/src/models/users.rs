use anyhow::anyhow;
use secrecy::Secret;
use time::OffsetDateTime;
use validator::Validate;

use configurations::hashed_password::compute_hashed_password;

use crate::models::base::{EmailAddress, EntityId};

/// ユーザー名の長さ
const USER_NAME_MIN_LEN: usize = 2;
const USER_NAME_MAX_LEN: usize = 40;

/// ユーザー名構造体
#[derive(Debug, Clone, Validate)]
pub struct UserName {
    #[validate(length(min = "USER_NAME_MIN_LEN", max = "USER_NAME_MAX_LEN"))]
    value: String,
}

impl UserName {
    /// ユーザー名インスタンスを構築する。
    ///
    /// # Arguments
    ///
    /// * `value` - ユーザー名。
    ///
    /// # Returns
    ///
    /// ユーザー名インスタンス。
    pub fn new(value: &str) -> anyhow::Result<Self> {
        let user_name = Self {
            value: value.to_owned(),
        };
        if user_name.validate().is_err() {
            return Err(anyhow!(format!(
                "ユーザー名は{}文字から{}文字です。",
                USER_NAME_MIN_LEN, USER_NAME_MAX_LEN
            )));
        }

        Ok(user_name)
    }

    /// ユーザー名を文字列で返却する。
    ///
    /// # Returns
    ///
    /// ユーザー名。
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// パスワードの最小文字数
const RAW_PASSWORD_MIN_LEN: usize = 8;
// パスワードに使用できる記号文字
const RAW_PASSWORD_SIGNS: &str = r##" !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~"##;

/// パスワード構造体
///
/// パスワードは、アルファベットの大文字と小文字、数字及び記号で構成された、8文字以上の文字列
/// でなければならない。
#[derive(Debug, Clone)]
pub struct RawPassword {
    value: Secret<String>,
}

impl RawPassword {
    /// コンストラクタ。
    ///
    /// # Arguments
    ///
    /// * `value` - パスワード。
    ///
    /// # Returns
    ///
    /// パスワード。
    pub fn new(value: &str) -> anyhow::Result<Self> {
        if value.len() < RAW_PASSWORD_MIN_LEN {
            return Err(anyhow!(format!(
                "パスワードは{}文字以上の文字列で指定してください。",
                RAW_PASSWORD_MIN_LEN
            )));
        }
        if !value.chars().any(|ch| ch.is_ascii_alphabetic()) {
            return Err(anyhow!("パスワードにアルファベットが含まれていません。"));
        }
        if !value.chars().any(|ch| ch.is_ascii_lowercase()) {
            return Err(anyhow!(
                "パスワードに小文字のアルファベットが含まれていません。"
            ));
        }
        if !value.chars().any(|ch| ch.is_ascii_uppercase()) {
            return Err(anyhow!(
                "パスワードに大文字のアルファベットが含まれていません。"
            ));
        }
        if !value.chars().any(|ch| ch.is_ascii_digit()) {
            return Err(anyhow!("パスワードに数字が含まれていません。"));
        }
        if !value.chars().any(|ch| RAW_PASSWORD_SIGNS.contains(ch)) {
            return Err(anyhow!("パスワードに記号が含まれていません。"));
        }

        Ok(Self {
            value: Secret::new(value.to_owned()),
        })
    }

    /// パスワードを返却する。
    ///
    /// # Returns
    ///
    /// * パスワード。
    pub fn value(&self) -> &Secret<String> {
        &self.value
    }
}

/// ハッシュ化パスワード構造体
#[derive(Debug, Clone)]
pub struct HashedPassword {
    value: Secret<String>,
}

impl HashedPassword {
    /// ハッシュ化パスワードインスタンスを構築する。
    ///
    /// # Arguments
    ///
    /// * `password`: パスワードインスタンス。
    ///
    /// # Returns
    ///
    /// ハッシュ化パスワードインスタンス。
    pub fn new(password: &RawPassword) -> anyhow::Result<Self> {
        let value = compute_hashed_password(password.value())?;

        Ok(Self { value })
    }

    /// ハッシュ化した文字列からハッシュ化パスワードインスタンスを構築する。
    ///
    /// # Arguments
    ///
    /// * `hashed_password`: パスワードインスタンス。
    ///
    /// # Returns
    ///
    /// ハッシュ化パスワードインスタンス。
    pub fn new_unchecked(hashed_password: &str) -> Self {
        Self {
            value: Secret::new(hashed_password.to_owned()),
        }
    }

    /// パスワードをハッシュ化したPHC文字列を返却する。
    ///
    /// # Returns
    ///
    /// パスワードをハッシュ化したPHC文字列。
    pub fn value(&self) -> &Secret<String> {
        &self.value
    }
}

/// ユーザーID
pub type UserId = EntityId<User>;

/// ユーザー
#[derive(Debug, Clone, Validate)]
pub struct User {
    /// ユーザーID。
    id: UserId,
    /// ユーザー名。
    user_name: UserName,
    /// Eメールアドレス。
    email_address: EmailAddress,
    /// ハッシュ化パスワード。
    hashed_password: HashedPassword,
    /// アクティブフラグ。
    is_active: bool,
    /// 最終ログイン日時。
    last_logged_in: Option<OffsetDateTime>,
    /// 作成日時。
    created_at: Option<OffsetDateTime>,
    /// 更新日時。
    updated_at: Option<OffsetDateTime>,
}

impl User {
    /// ユーザーインスタンスを構築する。
    ///
    /// # Arguments
    ///
    /// * `id` - ユーザーID。
    /// * `user_name` - ユーザー名。
    /// * `email_address` - Eメイルアドレス。
    /// * `hashed_password` - ハッシュ化パスワード。
    /// * `is_active` - アクティブフラグ。
    /// * `last_logged_in` - 最終ログイン日時。
    /// * `created_at` - 作成日時。
    /// * `updated_at` - 更新日時。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: UserId,
        user_name: UserName,
        email_address: EmailAddress,
        hashed_password: HashedPassword,
        is_active: bool,
        last_logged_in: Option<OffsetDateTime>,
        created_at: Option<OffsetDateTime>,
        updated_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            id,
            user_name,
            email_address,
            hashed_password,
            is_active,
            last_logged_in,
            created_at,
            updated_at,
        }
    }

    /// ユーザーIDを返却する。
    ///
    /// # Returns
    ///
    /// ユーザーIDインスタンス。
    pub fn id(&self) -> UserId {
        self.id.clone()
    }

    /// ユーザー名を返却する。
    ///
    /// # Returns
    ///
    /// ユーザー名インスタンス。
    pub fn user_name(&self) -> &UserName {
        &self.user_name
    }

    /// Eメールアドレスを返却する。
    ///
    /// # Returns
    ///
    /// Eメールアドレスインスタンス。
    pub fn email_address(&self) -> &EmailAddress {
        &self.email_address
    }

    /// ハッシュ化されたパスワードを返却する。
    ///
    /// # Returns
    ///
    /// ハッシュ化パスワードインスタンス。
    pub fn hashed_password(&self) -> &HashedPassword {
        &self.hashed_password
    }

    /// アクティブフラグを返却する。
    ///
    /// # Returns
    ///
    /// アクティブフラグ。
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// 最終ログイン日時を返却する。
    ///
    /// # Returns
    ///
    /// 最終ログイン日時。
    pub fn last_logged_in(&self) -> &Option<OffsetDateTime> {
        &self.last_logged_in
    }

    /// 作成日時を返却する。
    ///
    /// # Returns
    ///
    /// 作成ログイン日時。
    pub fn created_at(&self) -> &Option<OffsetDateTime> {
        &self.created_at
    }

    /// 更新日時を返却する。
    ///
    /// # Returns
    ///
    /// 更新ログイン日時。
    pub fn updated_at(&self) -> &Option<OffsetDateTime> {
        &self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn test_user_name_gen() {
        let values = vec!["x".repeat(USER_NAME_MIN_LEN), "x".repeat(USER_NAME_MAX_LEN)];
        for value in values {
            let user_name = UserName::new(&value);
            assert!(user_name.is_ok(), "{}", value);
            assert_eq!(user_name.unwrap().value(), value, "{}", value);
        }
    }

    #[test]
    fn test_user_name_gen_by_invalid_strings() {
        let values = vec![
            "x".repeat(USER_NAME_MIN_LEN - 1),
            "x".repeat(USER_NAME_MAX_LEN + 1),
        ];
        for value in values {
            let user_name = UserName::new(&value);
            assert!(user_name.is_err(), "{}", value);
        }
    }

    /// パスワードを構築できることを確認する。
    #[test]
    fn test_raw_password_gen() {
        let valid_password = "01abCD#$";
        let result = RawPassword::new(valid_password);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value().expose_secret(), valid_password);
    }

    /// パスワードを構築できないことを確認する。
    #[test]
    fn test_raw_password_new_invalid() {
        // 7文字
        assert!(RawPassword::new("01abCD#").is_err(), "パスワードの文字数");
        // アルファベットを含んでいない
        assert!(RawPassword::new("012345#$").is_err(), "アルファベット");
        // 大文字のファルファベットを含んでいない
        assert!(
            RawPassword::new("01abcd#$").is_err(),
            "大文字アルファベット"
        );
        // 小文字のファルファベットを含んでいない
        assert!(
            RawPassword::new("01ABCD#$").is_err(),
            "小文字アルファベット"
        );
        // 数字を含んでいない
        assert!(RawPassword::new("abcDEF#$").is_err(), "数字");
        // 記号を含んでいない
        assert!(RawPassword::new("01abCDef").is_err(), "記号");
    }
}
