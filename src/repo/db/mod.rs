//! # GitQLite Database Interface
//!
//! This module implements the interface between gitqlite models and the SQLite database.
//! It provides data structures and functions for handling various Git objects such as
//! blobs, trees, commits, and references.
//!
//! ## Key Components
//!
//! - `Index`: Represents the staging area of the Git repository.
//! - `Head`: Represents the current HEAD of the repository.
//! - `Ref`: Represents a Git reference (e.g., branches, tags).
//! - `Commit`: Represents a Git commit.
//! - `Tree`: Represents a Git tree object.
//! - `Blob`: Represents a Git blob object.
//! - `Sha1Id`: Represents a SHA1 hash used as an identifier for Git objects.
//!
//! ## Database Schema
//!
//! The module defines constants for creating the following tables:
//!
//! - `Index_`: Stores the staging area data.
//! - `Head`: Stores the current HEAD information.
//! - `Refs`: Stores Git references.
//! - `Commits`: Stores commit information.
//! - `Trees`: Stores tree object data.
//! - `Blobs`: Stores blob object data.
//!
//! ## Traits
//!
//! - `Hashable`: A trait for objects that can be hashed to generate an ID.
//! - `IdType`: A trait for handling different ID types (e.g., `NoId`, `Sha1Id`).
//!
//! ## Usage
//!
//! This module provides methods for reading from and writing to the SQLite database
//! for various Git objects. It also includes functionality for hashing objects and
//! managing the staging area.
//!
//! Note: This module relies on the `rusqlite` crate for SQLite database operations
//! and the `sha1` crate for hash computations.
//!

mod index;
mod object;

use std::fmt;

use anyhow::{anyhow, Context as _};

/// [`IdType`] represents a possible ID state of any object. In reality, object come from two sources:
/// 1. Top-down from querying the database (e.g., `gitqlite ls-file`).
/// 2. Bottom-up construction from the index, or arbitrary files/data (e.g., `git hash-object`)  
/// In case 1, the object always has a valid [`Sha1Id`], whereas in case 2, the object has [`NoId`], and needs
/// to be hashed before it could be persisted into the database.
pub trait IdType<T>: Copy + fmt::Display {
    type Id: PartialEq + Eq;

    #[allow(dead_code)]
    /// Returns the inner ID.
    fn id(self) -> Self::Id;
}

/// Generic trait describing any git object that could be hashed and get an ID for.
pub trait Hashable {
    fn hash(&self, sha: sha1::Sha1) -> Sha1Id;
}

/// [`NoId`] represents the state that an object does not have an SHA1 hash yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoId;

impl fmt::Display for NoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<NO ID>")
    }
}

impl<T> IdType<T> for NoId {
    type Id = std::convert::Infallible;

    fn id(self) -> Self::Id {
        unreachable!("You mustn't try to access non-IDs.");
    }
}

/// [`Sha1Id`] represents a sha1 hash
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sha1Id([u8; 20]);

impl fmt::Display for Sha1Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

impl serde::Serialize for Sha1Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as the hex string representation
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Sha1Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Sha1IdVisitor;
        impl<'v> serde::de::Visitor<'v> for Sha1IdVisitor {
            type Value = Sha1Id;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("A hex string of length 40")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Sha1Id::try_from(v) {
                    Ok(sha1_id) => Ok(sha1_id),
                    Err(e) => Err(E::custom(format!("{}", e))),
                }
            }
        }

        let s = deserializer.deserialize_str(Sha1IdVisitor)?;
        Ok(s)
    }
}

impl TryFrom<&str> for Sha1Id {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 40 {
            return Err(anyhow!("Invalid sha1 string: {}", value));
        }

        let mut bytes: [u8; 20] = [0; 20];

        for idx in (0..40).step_by(2) {
            let byte =
                u8::from_str_radix(&value[idx..idx + 2], 16).context("Converting str to Sha1Id")?;
            bytes[idx / 2] = byte;
        }

        Ok(Sha1Id(bytes))
    }
}

impl TryFrom<Vec<u8>> for Sha1Id {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let Ok(bytes) = value.try_into() else {
            return Err(anyhow!("Invalid sha1 byte"));
        };
        Ok(Sha1Id(bytes))
    }
}

impl IdType<Sha1Id> for Sha1Id {
    type Id = Sha1Id;

    fn id(self) -> Self::Id {
        self
    }
}

impl rusqlite::types::FromSql for Sha1Id {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let inner = <[u8; 20] as rusqlite::types::FromSql>::column_result(value)?;
        Ok(Sha1Id(inner))
    }
}

impl rusqlite::ToSql for Sha1Id {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}
