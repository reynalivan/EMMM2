use sqlx::database::Database;
use sqlx::decode::Decode;
use sqlx::encode::{Encode, IsNull};
use sqlx::error::BoxDynError;
use sqlx::{Sqlite, Type};

use crate::modules::games::domain::models::{GameType, ItemStatus};

impl Type<Sqlite> for GameType {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
        <i64 as Type<Sqlite>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for GameType {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        let raw = <i64 as Decode<Sqlite>>::decode(value)?;
        GameType::from_repr(raw as i32)
            .ok_or_else(|| format!("Unknown game type discriminant: {raw}").into())
    }
}

impl<'q> Encode<'q, Sqlite> for GameType {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <i64 as Encode<Sqlite>>::encode(*self as i64, buf)
    }
}

impl Type<Sqlite> for ItemStatus {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
        <i64 as Type<Sqlite>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for ItemStatus {
    fn decode(value: <Sqlite as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
        match <i64 as Decode<Sqlite>>::decode(value)? {
            0 => Ok(ItemStatus::Disabled),
            1 => Ok(ItemStatus::Enabled),
            raw => Err(format!("Unknown item status discriminant: {raw}").into()),
        }
    }
}

impl<'q> Encode<'q, Sqlite> for ItemStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        <i64 as Encode<Sqlite>>::encode(*self as i64, buf)
    }
}
