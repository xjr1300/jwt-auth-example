use anyhow::anyhow;
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

#[cfg(test)]
mod tests {
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
}
