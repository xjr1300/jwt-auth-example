use std::marker::PhantomData;

use anyhow::anyhow;
use uuid::Uuid;
use validator::Validate;

/// エンティティID構造体
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId<T> {
    value: Uuid,
    _marker: PhantomData<T>,
}

impl<T> EntityId<T> {
    /// 文字列からエンティティ構造体を構築する。
    ///
    /// # Arguments
    ///
    /// * value: IDに設定するUUID。
    ///
    /// # Returns
    ///
    /// エンティティIDインスタンス。
    pub fn gen(value: Uuid) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    /// IDをUUIDで返却する。
    ///
    /// # Returns
    ///
    /// ID。
    pub fn value(&self) -> &Uuid {
        &self.value
    }
}

impl<T> TryFrom<&str> for EntityId<T> {
    type Error = anyhow::Error;

    /// 文字列からエンティティIDを構築する。
    ///
    /// # Arguments:
    ///
    /// * `value` - エンティティIDを構築する文字列。
    ///
    /// # Returns
    ///
    /// エンティティIDインスタンス。
    fn try_from(value: &str) -> anyhow::Result<Self, Self::Error> {
        Uuid::try_parse(value)
            .map(|value| Ok(Self::gen(value)))
            .map_err(|err| anyhow!("{:?}", err))?
    }
}

/// Eメールアドレス構造体
#[derive(Debug, Clone, Validate)]
pub struct EmailAddress {
    #[validate(email)]
    value: String,
}

impl EmailAddress {
    /// Eメールアドレスインスタンスを生成する。
    ///
    /// # Arguments
    ///
    /// * `value` - Eメールアドレス。
    ///
    /// # Returns
    ///
    /// Eメールアドレスインスタンス。
    pub fn gen(value: &str) -> anyhow::Result<Self> {
        let email = Self {
            value: value.to_owned(),
        };
        if email.validate().is_err() {
            return Err(anyhow!(format!("Eメールアドレス({})が不正です。", value)));
        }

        Ok(email)
    }

    /// Eメールアドレスを文字列で返却する。
    ///
    /// # Returns
    ///
    /// Eメールアドレス。
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_try_from() {
        let uuid = uuid::Uuid::new_v4();
        let value = uuid.to_string();
        let id = EntityId::<i32>::try_from(value.as_str());
        assert!(id.is_ok());
    }

    #[test]
    fn test_entity_id_try_from_by_invalid_string() {
        let value = "this-is-invalid-uuid";
        let id = EntityId::<i32>::try_from(value);
        assert!(id.is_err());
    }

    #[test]
    fn test_gen_email_address() {
        // https://gist.github.com/cjaoude/fd9910626629b53c4d25
        /* cSpell: disable */
        let values = vec![
            "email@example.com",
            "firstname.lastname@example.com",
            "email@subdomain.example.com",
            "firstname+lastname@example.com",
            "email@123.123.123.123",
            "email@[123.123.123.123]",
            // r#"email"@example.com"#,
            "1234567890@example.com",
            "email@example-one.com",
            "_______@example.com",
            "email@example.name",
            "email@example.museum",
            "email@example.co.jp",
            "firstname-lastname@example.com",
            // r#"much.”more\ unusual”@example.com"#,
            // r#"very.unusual.”@”.unusual.com@example.com"#,
            // r#"very.”(),:;<>[]”.VERY.”very@\\ "very”.unusual@strange.example.com"#,
        ];
        /* cSpell: enable */
        for value in values {
            let email = EmailAddress::gen(value);
            assert!(email.is_ok(), "{}", value);
            assert_eq!(email.unwrap().value(), value, "{}", value);
        }
    }

    #[test]
    fn test_email_address_gen_by_invalid_strings() {
        /* cSpell: disable */
        // https://gist.github.com/cjaoude/fd9910626629b53c4d25
        let values = vec![
            // "plainaddress",
            // "#@%^%#$@#$@#.com",
            "@example.com",
            "Joe Smith <email@example.com>",
            "email.example.com",
            "email@example@example.com",
            // ".email@example.com",
            // "email.@example.com",
            // "email..email@example.com",
            "あいうえお@example.com",
            "email@example.com (Joe Smith)",
            // "email@example",
            "email@-example.com",
            // "email@example.web",
            // "email@111.222.333.44444",
            "email@example..com",
            // "Abc..123@example.com",
        ];
        /* cSpell: enable */
        for value in values {
            let email = EmailAddress::gen(value);
            assert!(email.is_err(), "{}", value);
        }
    }
}
