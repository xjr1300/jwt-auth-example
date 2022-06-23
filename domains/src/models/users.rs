use anyhow::anyhow;
use secrecy::Secret;
use validator::Validate;

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
    pub fn gen(value: &str) -> anyhow::Result<Self> {
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
    pub fn gen(value: &str) -> anyhow::Result<Self> {
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

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    #[test]
    fn test_user_name_gen() {
        let values = vec!["x".repeat(USER_NAME_MIN_LEN), "x".repeat(USER_NAME_MAX_LEN)];
        for value in values {
            let user_name = UserName::gen(&value);
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
            let user_name = UserName::gen(&value);
            assert!(user_name.is_err(), "{}", value);
        }
    }

    /// パスワードを構築できることを確認する。
    #[test]
    fn test_raw_password_gen() {
        let valid_password = "01abCD#$";
        let result = RawPassword::gen(valid_password);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value().expose_secret(), valid_password);
    }

    /// パスワードを構築できないことを確認する。
    #[test]
    fn test_raw_password_new_invalid() {
        // 7文字
        assert!(RawPassword::gen("01abCD#").is_err(), "パスワードの文字数");
        // アルファベットを含んでいない
        assert!(RawPassword::gen("012345#$").is_err(), "アルファベット");
        // 大文字のファルファベットを含んでいない
        assert!(
            RawPassword::gen("01abcd#$").is_err(),
            "大文字アルファベット"
        );
        // 小文字のファルファベットを含んでいない
        assert!(
            RawPassword::gen("01ABCD#$").is_err(),
            "小文字アルファベット"
        );
        // 数字を含んでいない
        assert!(RawPassword::gen("abcDEF#$").is_err(), "数字");
        // 記号を含んでいない
        assert!(RawPassword::gen("01abCDef").is_err(), "記号");
    }
}
