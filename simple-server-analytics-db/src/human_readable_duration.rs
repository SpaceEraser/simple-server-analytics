use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
    time::Duration,
};

use sqlx::{
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
    Decode, Encode, Sqlite, Type,
};

const NANOS_PER_YEAR: u128 = 1_000_000_000 * 60 * 60 * 24 * 365;
const NANOS_PER_DAY: u128 = 1_000_000_000 * 60 * 60 * 24;
const NANOS_PER_HOUR: u128 = 1_000_000_000 * 60 * 60;
const NANOS_PER_MIN: u128 = 1_000_000_000 * 60;
const NANOS_PER_SEC: u128 = 1_000_000_000;

pub struct HumanReadableDuration(pub Duration);

impl Display for HumanReadableDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut spacer = "";

        let years = self.0.as_nanos() / NANOS_PER_YEAR;
        if years > 0 {
            writeln!(f, "{years}y")?;
            spacer = " ";
        }

        let days = (self.0.as_nanos() % NANOS_PER_YEAR) / NANOS_PER_DAY;
        if days > 0 {
            writeln!(f, "{spacer}{days}d")?;
            spacer = " ";
        }

        let hours = (self.0.as_nanos() % NANOS_PER_DAY) / NANOS_PER_HOUR;
        if hours > 0 {
            writeln!(f, "{spacer}{hours}h")?;
            spacer = " ";
        }

        let mins = (self.0.as_nanos() % NANOS_PER_HOUR) / NANOS_PER_MIN;
        if mins > 0 {
            writeln!(f, "{spacer}{mins}m")?;
            spacer = " ";
        }

        let secs = (self.0.as_nanos() % NANOS_PER_MIN) as f64 / NANOS_PER_SEC as f64;
        if secs > f64::EPSILON {
            writeln!(f, "{spacer}{secs}s")?;
        }

        Ok(())
    }
}

impl FromStr for HumanReadableDuration {
    type Err = Box<dyn StdError + 'static + Send + Sync>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut dur = Duration::default();

        for part in s.split_ascii_whitespace() {
            if part.ends_with('y') {
                dur += Duration::from_secs(365 * 24 * 60 * 60)
            }
            if part.ends_with('d') {
                dur += Duration::from_secs(24 * 60 * 60)
            }
            if part.ends_with('h') {
                dur += Duration::from_secs(60 * 60)
            }
            if part.ends_with('m') {
                dur += Duration::from_secs(60)
            }
            if part.ends_with('s') {
                dur += Duration::from_secs_f64(part[..part.len() - 1].parse()?)
            }
        }

        Ok(HumanReadableDuration(dur))
    }
}

impl Encode<'_, Sqlite> for HumanReadableDuration {
    fn encode_by_ref(&self, buf: &mut Vec<SqliteArgumentValue<'_>>) -> sqlx::encode::IsNull {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Decode<'_, Sqlite> for HumanReadableDuration {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, sqlx::error::BoxDynError> {
        match <String as Decode<Sqlite>>::decode(value) {
            Ok(s) => Ok(s.parse::<Self>().unwrap()),
            Err(e) => Err(e),
        }
    }
}

impl Type<Sqlite> for HumanReadableDuration {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}
