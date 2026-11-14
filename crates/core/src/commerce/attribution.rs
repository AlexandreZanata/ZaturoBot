//! Bounded explicit sub-ID associations; this module never edits an affiliate URL.
use super::offer::AffiliateProgram;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributionError {
    InvalidBounds,
    InvalidContentId,
    InvalidSubId,
    SubIdLimit,
    Capacity,
    Conflict,
}
impl fmt::Display for AttributionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBounds => "attribution bounds are outside reviewed limits",
            Self::InvalidContentId => "content identity is invalid",
            Self::InvalidSubId => "official sub-ID is not bounded URL-safe text",
            Self::SubIdLimit => "the complete generated identifier exceeds the program limit",
            Self::Capacity => "attribution map capacity is exhausted",
            Self::Conflict => "attribution identity has a conflicting association",
        })
    }
}
impl std::error::Error for AttributionError {}

/// Pure per-program bijection. Caller must verify that the program permits sub-IDs and
/// supply its documented byte limit; storing a mapping does not grant tracking rights.
#[derive(Debug, Clone)]
pub struct AttributionMap {
    program: AffiliateProgram,
    sub_id_limit: usize,
    max_entries: usize,
    by_sub_id: BTreeMap<String, String>,
    by_content: BTreeMap<String, String>,
}
impl AttributionMap {
    pub fn new(
        program: AffiliateProgram,
        sub_id_limit: usize,
        max_entries: usize,
    ) -> Result<Self, AttributionError> {
        if !(1..=128).contains(&sub_id_limit) || !(1..=1000).contains(&max_entries) {
            return Err(AttributionError::InvalidBounds);
        }
        Ok(Self {
            program,
            sub_id_limit,
            max_entries,
            by_sub_id: BTreeMap::new(),
            by_content: BTreeMap::new(),
        })
    }

    /// Records an exact official association, refusing collisions in either direction.
    pub fn register(&mut self, content_id: &str, sub_id: &str) -> Result<(), AttributionError> {
        crate::validate_identifier(content_id).map_err(|_| AttributionError::InvalidContentId)?;
        if sub_id.is_empty()
            || sub_id.len() > self.sub_id_limit
            || !sub_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(AttributionError::InvalidSubId);
        }
        if let Some(existing) = self.by_content.get(content_id) {
            return if existing == sub_id {
                Ok(())
            } else {
                Err(AttributionError::Conflict)
            };
        }
        if self.by_sub_id.contains_key(sub_id) {
            return Err(AttributionError::Conflict);
        }
        if self.by_content.len() >= self.max_entries {
            return Err(AttributionError::Capacity);
        }
        self.by_content.insert(content_id.into(), sub_id.into());
        self.by_sub_id.insert(sub_id.into(), content_id.into());
        Ok(())
    }

    /// Proposes a full program-namespaced SHA-256 identifier. Never truncate a digest to
    /// satisfy a program limit; official link creation remains an authorized later step.
    pub fn generate(&mut self, content_id: &str) -> Result<String, AttributionError> {
        crate::validate_identifier(content_id).map_err(|_| AttributionError::InvalidContentId)?;
        if let Some(existing) = self.by_content.get(content_id) {
            return Ok(existing.clone());
        }
        if self.sub_id_limit < 64 {
            return Err(AttributionError::SubIdLimit);
        }
        let mut hash = Sha256::new();
        hash.update(self.program.as_str().as_bytes());
        hash.update([0]);
        hash.update(content_id.as_bytes());
        let sub_id = format!("{:x}", hash.finalize());
        self.register(content_id, &sub_id)?;
        Ok(sub_id)
    }

    /// Unknown source-report IDs remain unattributed; no guessing or fuzzy matching.
    pub fn lookup(&self, sub_id: &str) -> Option<&str> {
        self.by_sub_id.get(sub_id).map(String::as_str)
    }

    pub fn sub_id_for(&self, content_id: &str) -> Option<&str> {
        self.by_content.get(content_id).map(String::as_str)
    }
}
