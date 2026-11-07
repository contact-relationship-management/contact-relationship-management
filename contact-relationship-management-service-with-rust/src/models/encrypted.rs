//! [`EncryptedText`]: a `String` column that is sealed on the way into the
//! database and opened on the way out (CRM-T41), so every query and
//! handler keeps working with plaintext and none can forget to encrypt.
//!
//! In memory and in JSON it is the plaintext; in the database it is the
//! `enc:v1:` form from [`crate::crypto`] (or legacy plaintext when no keys
//! are configured). Raw SQL bypasses this type, so a raw write stores
//! plaintext, which still reads back (and `rotate-field-keys` then seals it).
//!
//! `sqlx` goes through the same two functions: binding this type seals, decoding it opens, and a
//! value that cannot be opened is an error, never ciphertext shown as text (CRM-D24). `sqlx.toml`
//! makes the macros read both columns as this type, but it covers reads only, so a write must bind
//! `EncryptedText` itself. A census test in `db.rs` holds which modules write them.

use std::ops::Deref;

use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, Value, ValueType, ValueTypeErr};
use sea_orm::{ActiveValue, DbErr, IntoActiveValue, QueryResult, TryGetError, TryGetable};
use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, Postgres, Type};

use crate::crypto;

/// Plaintext in the program, ciphertext in the database.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EncryptedText(pub String);

impl schemars::JsonSchema for EncryptedText {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "EncryptedText".into()
    }
    fn inline_schema() -> bool {
        true
    }
    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <String as schemars::JsonSchema>::json_schema(generator)
    }
}

impl EncryptedText {
    /// The plaintext.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for EncryptedText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for EncryptedText {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EncryptedText {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<EncryptedText> for Value {
    fn from(value: EncryptedText) -> Self {
        Self::String(Some(crypto::seal(&value.0)))
    }
}

impl ValueType for EncryptedText {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::String(Some(stored)) => {
                crypto::open(&stored).map(Self).map_err(|_| ValueTypeErr)
            }
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        "EncryptedText".to_string()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::Text
    }
}

impl Nullable for EncryptedText {
    fn null() -> Value {
        Value::String(None)
    }
}

impl TryGetable for EncryptedText {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        let stored = <String as TryGetable>::try_get_by(res, index)?;
        crypto::open(&stored)
            .map(Self)
            .map_err(|error| TryGetError::DbErr(DbErr::Type(error.to_string())))
    }
}

impl Type<Postgres> for EncryptedText {
    fn type_info() -> PgTypeInfo {
        <String as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<Postgres>>::compatible(ty)
    }
}

impl Encode<'_, Postgres> for EncryptedText {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Postgres>>::encode(crypto::seal(&self.0), buf)
    }
}

impl Decode<'_, Postgres> for EncryptedText {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        let stored = <String as Decode<'_, Postgres>>::decode(value)?;
        Ok(Self(crypto::open(&stored)?))
    }
}

impl IntoActiveValue<Self> for EncryptedText {
    fn into_active_value(self) -> ActiveValue<Self> {
        ActiveValue::Set(self)
    }
}
