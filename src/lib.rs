//! # HashMemo - Hash Value Memoization
//!
//! A library for memoizing hash values of complex data structures to improve performance
//! when the same data is hashed multiple times.
//!
//! This library is particularly beneficial for **large data structures** where computing
//! hash values is expensive (e.g., large strings, vectors, complex nested structures).
//! By caching the hash value after first computation, subsequent hash operations become
//! O(1) instead of O(n) where n is the size of the data.
//!
//! ## Features
//!
//! - Lazy hash computation - only calculates when needed
//! - Thread-safe caching with atomic operations  
//! - Minimal memory overhead with zero-sized hashers
//! - Works with any `BuildHasher` implementation
//!
//! ## Examples
//!
//! ```rust
//! use hashmemo::HashMemo;
//! use std::collections::HashMap;
//!
//! // Wrap a large string that will be hashed multiple times
//! let large_data = "a".repeat(10000); // 10KB string
//! let memo = HashMemo::new(large_data);
//!
//! // Use in HashMap - hash is computed once and cached
//! let mut map = HashMap::new();
//! map.insert(memo, "value");
//!
//! // Subsequent operations using the same data are much faster
//! // because the hash is already cached
//! ```
//!
//! ## Performance Benefits
//!
//! The performance improvement is most significant for:
//! - Large strings or byte arrays
//! - Complex nested data structures (Vec, HashMap, etc.)
//! - Data that will be used as hash keys multiple times
//! - Concurrent scenarios where the same data is hashed by multiple threads

use std::borrow::Borrow;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// A wrapper that memoizes the hash value of its contained data.
#[derive(Debug)]
pub struct HashMemo<T, H: BuildHasher = BuildHasherDefault<DefaultHasher>>
where
    T: Eq + PartialEq + Hash,
{
    value: T,
    hash: AtomicU64,
    hasher: H,
}

impl<T, H> PartialOrd for HashMemo<T, H>
where
    T: Eq + Hash + PartialOrd,
    H: BuildHasher,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T, H> Ord for HashMemo<T, H>
where
    T: Eq + Hash + Ord,
    H: BuildHasher,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> HashMemo<T, BuildHasherDefault<DefaultHasher>>
where
    T: Eq + Hash,
{
    /// Creates a new `HashMemo` with the default hasher.
    ///
    /// The default hasher is `BuildHasherDefault<DefaultHasher>`, which is a zero-sized
    /// type that creates `DefaultHasher` instances.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hashmemo::HashMemo;
    ///
    /// let memo = HashMemo::new("hello world");
    /// ```
    pub fn new(value: T) -> Self {
        Self::with_hasher(value, BuildHasherDefault::default())
    }
}

impl<T, H> HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
    /// Creates a new `HashMemo` with a custom hasher.
    ///
    /// This allows you to specify a custom `BuildHasher` implementation for
    /// controlling how hash values are computed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hashmemo::HashMemo;
    /// use std::hash::BuildHasherDefault;
    /// use std::collections::hash_map::DefaultHasher;
    ///
    /// let memo = HashMemo::with_hasher("hello", BuildHasherDefault::<DefaultHasher>::default());
    /// ```
    pub const fn with_hasher(value: T, hasher: H) -> Self {
        Self {
            value,
            hash: AtomicU64::new(u64::MIN),
            hasher,
        }
    }

    /// Consumes the `HashMemo` and returns the wrapped value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use hashmemo::HashMemo;
    ///
    /// let memo = HashMemo::new("hello");
    /// let value = memo.into_inner();
    /// assert_eq!(value, "hello");
    /// ```
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, H> PartialEq for HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T, H> Eq for HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
}

impl<T, H> Hash for HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
    fn hash<H2: Hasher>(&self, state: &mut H2) {
        let hash = self.hash.load(Ordering::Relaxed);
        if hash != 0 {
            state.write_u64(hash);
            return;
        }

        let computed_hash = NonZeroU64::new(self.hasher.hash_one(&self.value))
            .map(NonZeroU64::get)
            .unwrap_or(u64::MIN | 1);

        let _ = self.hash.compare_exchange(
            u64::MIN,
            computed_hash,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        state.write_u64(computed_hash);
    }
}

impl<T, H> AsRef<T> for HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T, H> Borrow<T> for HashMemo<T, H>
where
    T: Eq + Hash,
    H: BuildHasher,
{
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T, H> From<T> for HashMemo<T, BuildHasherDefault<H>>
where
    T: Eq + Hash,
    H: Hasher + Default,
{
    fn from(value: T) -> Self {
        Self::with_hasher(value, BuildHasherDefault::<H>::default())
    }
}

impl<T, H> Clone for HashMemo<T, H>
where
    T: Eq + Hash + Clone,
    H: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            hash: AtomicU64::new(self.hash.load(Ordering::Relaxed)),
            hasher: self.hasher.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use super::*;

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        calculate_hash_with_hasher::<T, DefaultHasher>(t)
    }

    fn calculate_hash_with_hasher<T: Hash, H: Hasher + Default>(t: &T) -> u64 {
        let mut s = H::default();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn hash_is_stable_after_clone() {
        let foo = HashMemo::new("foo".to_string());
        let hash = calculate_hash(&foo);

        let foo2 = foo.clone();
        let hash2 = calculate_hash(&foo2);

        assert_eq!(hash, hash2, "Hash should remain the same after cloning");
    }

    #[test]
    fn hash_is_consistent_on_reuse() {
        let foo = HashMemo::new("foo".to_string());
        let hash1 = calculate_hash(&foo);
        let hash2 = calculate_hash(&foo);
        let hash3 = calculate_hash(&foo);

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn hash_is_cached_and_only_calculated_once() {
        struct HashOnce {
            hashed_once: Arc<AtomicBool>,
        }

        impl Eq for HashOnce {}
        impl PartialEq for HashOnce {
            fn eq(&self, _: &Self) -> bool {
                true
            }
        }

        impl Hash for HashOnce {
            fn hash<H: Hasher>(&self, _: &mut H) {
                if self.hashed_once.swap(true, Ordering::SeqCst) {
                    panic!("Hashing should only happen once");
                }
            }
        }

        let foo = HashMemo::new(HashOnce {
            hashed_once: Arc::new(AtomicBool::new(false)),
        });

        for _ in 0..10 {
            calculate_hash(&foo);
        }
    }

    #[test]
    fn into_inner_returns_original_value() {
        let foo = HashMemo::new("foo".to_string());
        let inner = HashMemo::into_inner(foo);
        assert_eq!(inner, "foo".to_string());
    }

    #[test]
    fn struct_is_not_significantly_larger_than_wrapped_value() {
        assert!(
            std::mem::size_of::<HashMemo<String>>()
                <= std::mem::size_of::<String>() + std::mem::size_of::<u64>(),
            "HashMemo should have minimal overhead"
        );
    }

    #[test]
    fn zero_hash_is_remapped_to_nonzero_in_cache() {
        use nohash_hasher::NoHashHasher;

        struct PinHash<const H: u64>();
        impl<const H: u64> Eq for PinHash<H> {}
        impl<const H: u64> PartialEq for PinHash<H> {
            fn eq(&self, _: &Self) -> bool {
                true
            }
        }
        impl<const H: u64> Hash for PinHash<H> {
            fn hash<HS: Hasher>(&self, state: &mut HS) {
                state.write_u64(H);
            }
        }

        // Sanity check: hash value of FixedHash<0> using dummy hasher is 0
        assert_eq!(
            calculate_hash_with_hasher::<PinHash<0>, NoHashHasher<u64>>(&PinHash::<0>()),
            0
        );

        // Wrap in HashMemo and ensure it's not stored as zero
        let memo: HashMemo<_, BuildHasherDefault<NoHashHasher<u64>>> = HashMemo::with_hasher(
            PinHash::<0>(),
            BuildHasherDefault::<NoHashHasher<u64>>::default(),
        );

        let _ = calculate_hash(&memo);
        let cached = memo.hash.load(Ordering::Relaxed);
        assert_ne!(cached, 0, "Cached hash must not be zero");
    }
}
