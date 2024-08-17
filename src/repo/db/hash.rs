use super::Sha1Id;

/// Generic trait describing any git object that could be hashed and get an ID for.
pub trait Hashable {
    fn hash(&self, sha: sha1::Sha1) -> Sha1Id;
}
