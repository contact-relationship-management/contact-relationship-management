//! [`Timestamp`] and [`Date`] as Postgres's `timestamptz` and `date`, behind the `postgres` feature
//! (CRM-T260). `sqlx` 0.9 has no `jiff` support, so these encode and decode the binary forms
//! (microseconds and days from 2000-01-01) and read the text form. The rules never use them; the
//! service turns the feature on, and its `sqlx.toml` names these types for every such column.

use sqlx::{
    Decode, Encode, Postgres, Type,
    encode::IsNull,
    error::BoxDynError,
    postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueFormat, PgValueRef},
};

use super::{Date, Timestamp};

impl Type<Postgres> for Timestamp {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("timestamptz")
    }
}

impl PgHasArrayType for Timestamp {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_timestamptz")
    }
}

impl Encode<'_, Postgres> for Timestamp {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(&self.to_pg_micros()?.to_be_bytes());
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        8
    }
}

impl Decode<'_, Postgres> for Timestamp {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        match value.format() {
            PgValueFormat::Binary => {
                let bytes: [u8; 8] = value.as_bytes()?.try_into()?;
                Ok(Self::from_pg_micros(i64::from_be_bytes(bytes))?)
            }
            PgValueFormat::Text => Ok(value.as_str()?.parse()?),
        }
    }
}

impl Type<Postgres> for Date {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("date")
    }
}

impl PgHasArrayType for Date {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_date")
    }
}

impl Encode<'_, Postgres> for Date {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(&self.days_from_pg_epoch()?.to_be_bytes());
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        4
    }
}

impl Decode<'_, Postgres> for Date {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        match value.format() {
            PgValueFormat::Binary => {
                let bytes: [u8; 4] = value.as_bytes()?.try_into()?;
                Ok(Self::from_pg_days(i32::from_be_bytes(bytes))?)
            }
            PgValueFormat::Text => Ok(value.as_str()?.parse()?),
        }
    }
}
