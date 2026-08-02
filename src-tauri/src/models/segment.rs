use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use rusqlite::types::{FromSql, FromSqlResult, FromSqlError, ValueRef, ToSql, ToSqlOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStatus {
    Pending,
    Queued,
    Processing,
    Success,
    Approved,
    RetryWait,
    Failed,
    Stale,
}

impl SegmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Queued => "queued",
            Self::Processing => "processing",
            Self::Success => "success",
            Self::Approved => "approved",
            Self::RetryWait => "retry_wait",
            Self::Failed => "failed",
            Self::Stale => "stale",
        }
    }
}

impl fmt::Display for SegmentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SegmentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "queued" => Ok(Self::Queued),
            "processing" => Ok(Self::Processing),
            "success" => Ok(Self::Success),
            "approved" => Ok(Self::Approved),
            "retry_wait" => Ok(Self::RetryWait),
            "failed" => Ok(Self::Failed),
            "stale" => Ok(Self::Stale),
            other => Err(format!("Invalid SegmentStatus: '{}'", other)),
        }
    }
}

impl ToSql for SegmentStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for SegmentStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str()?.parse().map_err(|err_msg| FromSqlError::Other(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err_msg))))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Unreviewed,
    Approved,
    NeedsFix,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unreviewed => "unreviewed",
            Self::Approved => "approved",
            Self::NeedsFix => "needs_fix",
        }
    }
}

impl FromStr for ReviewStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unreviewed" => Ok(Self::Unreviewed),
            "approved" => Ok(Self::Approved),
            "needs_fix" => Ok(Self::NeedsFix),
            other => Err(format!("Invalid ReviewStatus: '{}'", other)),
        }
    }
}

impl ToSql for ReviewStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.as_str()))
    }
}

impl FromSql for ReviewStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str()?.parse().map_err(|err_msg| FromSqlError::Other(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err_msg))))
    }
}
