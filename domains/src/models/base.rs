use std::marker::PhantomData;

use anyhow::anyhow;
use uuid::Uuid;

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
    pub(crate) fn gen(value: Uuid) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_uuid_str() {
        let uuid = uuid::Uuid::new_v4();
        let value = uuid.to_string();
        let id = EntityId::<i32>::try_from(value.as_str());
        assert!(id.is_ok());
    }

    #[test]
    fn test_from_invalid_uuid_str() {
        let str = "this-is-invalid-uuid";
        let id = EntityId::<i32>::try_from(str);
        assert!(id.is_err());
    }
}
