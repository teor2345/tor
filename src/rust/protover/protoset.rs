// Copyright (c) 2018, The Tor Project, Inc.
// Copyright (c) 2018, isis agora lovecruft
// See LICENSE for licensing information

//! Sets for lazily storing ordered, non-overlapping ranges of integers.

use std::slice;
use std::str::FromStr;
use std::u32;

use errors::ProtoverError;

/// A single version number.
pub type Version = u32;

/// A `ProtoSet` stores an ordered `Vec<T>` of `(low, high)` pairs of ranges of
/// non-overlapping protocol versions.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
///
/// use protover::errors::ProtoverError;
/// use protover::protoset::ProtoSet;
/// use protover::protoset::Version;
///
/// # fn do_test() -> Result<ProtoSet, ProtoverError> {
/// let protoset: ProtoSet = ProtoSet::from_str("3-5,8")?;
///
/// // We could also equivalently call:
/// let protoset: ProtoSet = "3-5,8".parse()?;
///
/// assert!(protoset.contains(4));
/// assert!(!protoset.contains(7));
///
/// let expanded: Vec<Version> = protoset.expand().collect();
///
/// assert_eq!(&expanded[..], &[3, 4, 5, 8]);
///
/// let contracted: String = protoset.clone().to_string();
///
/// assert_eq!(contracted, "3-5,8".to_string());
/// # Ok(protoset)
/// # }
/// # fn main() { do_test(); }  // wrap the test so we can use the ? operator
#[derive(Default, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProtoSet {
    pub(crate) pairs: Vec<(Version, Version)>,
}

impl<'a> ProtoSet {
    /// Create a new `ProtoSet` from a slice of `(low, high)` pairs.
    ///
    /// # Inputs
    ///
    /// We do not assume the input pairs are deduplicated or ordered.
    pub fn from_slice(low_high_pairs: &'a [(Version, Version)]) -> Result<Self, ProtoverError> {
        let mut pairs: Vec<(Version, Version)> = Vec::with_capacity(low_high_pairs.len());

        for &(low, high) in low_high_pairs {
            pairs.push((low, high));
        }
        // Sort the pairs without reallocation and remove all duplicate pairs.
        pairs.sort_unstable();
        pairs.dedup();

        ProtoSet::from_sorted(pairs)
    }
}

impl ProtoSet {
    /// Get an iterator over the `(low, high)` `pairs` in this `ProtoSet`.
    pub fn iter(&self) -> slice::Iter<(Version, Version)> {
        self.pairs.iter()
    }

    /// Expand this `ProtoSet` into a `Vec` of all its `Version`s.
    ///
    /// # Examples
    ///
    /// ```
    /// # use protover::errors::ProtoverError;
    /// use protover::protoset::ProtoSet;
    ///
    /// # fn do_test() -> Result<bool, ProtoverError> {
    /// let protoset: ProtoSet = "3-5,9".parse()?;
    ///
    /// assert!(protoset.expand().eq(vec![3, 4, 5, 9]));
    ///
    /// let protoset: ProtoSet = "1,3,5-7".parse()?;
    ///
    /// assert!(protoset.expand().eq(vec![1, 3, 5, 6, 7]));
    /// #
    /// # Ok(true)
    /// # }
    /// # fn main() { do_test(); }  // wrap the test so we can use the ? operator
    /// ```
    pub fn expand(&self) -> impl DoubleEndedIterator<Item = Version> + '_ {
        self.iter()
            .map(|&(low, high)| low..high + 1)
            .flat_map(|x| x)
    }

    pub fn len(&self) -> usize {
        let mut length: usize = 0;

        for &(low, high) in self.iter() {
            length += (high as usize - low as usize) + 1;
        }

        length
    }

    /// Creates a `ProtoSet` from a vector. The vector must be already sorted.
    ///
    /// This is automatically called in `ProtoSet::from_str()`.
    ///
    /// # Errors
    ///
    /// * `ProtoverError::LowGreaterThanHigh`: if its `pairs` were not
    ///   well-formed, i.e. a `low` in a `(low, high)` was higher than the
    ///   previous `high`,
    /// * `ProtoverError::Overlap`: if one or more of the `pairs` are
    ///   overlapping,
    /// * `ProtoverError::ExceedsMax`: if the number of versions when expanded
    ///   would exceed `MAX_PROTOCOLS_TO_EXPAND`, and
    ///
    /// # Returns
    ///
    /// A `Result` whose `Ok` is a new `Protoset`, and whose `Err` is one of the
    /// errors enumerated in the Errors section above.
    fn from_sorted(pairs: Vec<(Version, Version)>) -> Result<ProtoSet, ProtoverError> {
        let mut last_high: Version = 0;

        for &(low, high) in &pairs {
            if low == u32::MAX || high == u32::MAX {
                return Err(ProtoverError::ExceedsMax);
            }
            if low < last_high {
                return Err(ProtoverError::Overlap);
            } else if low > high {
                return Err(ProtoverError::LowGreaterThanHigh);
            }
            last_high = high;
        }

        Ok(ProtoSet { pairs })
    }

    /// Determine if this `ProtoSet` contains no `Version`s.
    ///
    /// # Returns
    ///
    /// * `true` if this `ProtoSet`'s length is zero, and
    /// * `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use protover::protoset::ProtoSet;
    ///
    /// let protoset: ProtoSet = ProtoSet::default();
    ///
    /// assert!(protoset.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.pairs.len() == 0
    }

    /// Determine if `version` is included within this `ProtoSet`.
    ///
    /// # Inputs
    ///
    /// * `version`: a `Version`.
    ///
    /// # Returns
    ///
    /// `true` if the `version` is contained within this set; `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use protover::errors::ProtoverError;
    /// use protover::protoset::ProtoSet;
    ///
    /// # fn do_test() -> Result<ProtoSet, ProtoverError> {
    /// let protoset: ProtoSet = ProtoSet::from_slice(&[(0, 5), (7, 9), (13, 14)])?;
    ///
    /// assert!(protoset.contains(5));
    /// assert!(!protoset.contains(10));
    /// #
    /// # Ok(protoset)
    /// # }
    /// # fn main() { do_test(); }  // wrap the test so we can use the ? operator
    /// ```
    pub fn contains(&self, version: Version) -> bool {
        for &(low, high) in self.iter() {
            if low <= version && version <= high {
                return true;
            }
        }
        false
    }

    /// Retain only the `Version`s in this `ProtoSet` for which the predicate
    /// `F` returns `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use protover::errors::ProtoverError;
    /// use protover::protoset::ProtoSet;
    ///
    /// # fn do_test() -> Result<bool, ProtoverError> {
    /// let mut protoset: ProtoSet = "1,3-5,9".parse()?;
    ///
    /// // Keep only versions less than or equal to 8:
    /// protoset.retain(|&x| x <= 8);
    ///
    /// assert!(protoset.expand().eq(vec![1, 3, 4, 5]));
    /// #
    /// # Ok(true)
    /// # }
    /// # fn main() { do_test(); }  // wrap the test so we can use the ? operator
    /// ```
    // XXX we could probably do something more efficient here. —isis
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Version) -> bool,
    {
        let expanded: Vec<Version> = self.expand().filter(f).collect();
        *self = expanded.into();
    }
}

impl FromStr for ProtoSet {
    type Err = ProtoverError;

    /// Parse the unique version numbers supported by a subprotocol from a string.
    ///
    /// # Inputs
    ///
    /// * `version_string`, a string comprised of "[0-9,-]"
    ///
    /// # Returns
    ///
    /// A `Result` whose `Ok` value is a `ProtoSet` holding all of the unique
    /// version numbers.
    ///
    /// The returned `Result`'s `Err` value is an `ProtoverError` appropriate to
    /// the error.
    ///
    /// # Errors
    ///
    /// This function will error if:
    ///
    /// * the `version_string` is an equals (`"="`) sign,
    /// * the expansion of a version range produces an error (see
    ///   `expand_version_range`),
    /// * any single version number is not parseable as an `u32` in radix 10, or
    /// * there are greater than 2^16 version numbers to expand.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    ///
    /// use protover::errors::ProtoverError;
    /// use protover::protoset::ProtoSet;
    ///
    /// # fn do_test() -> Result<ProtoSet, ProtoverError> {
    /// let protoset: ProtoSet = ProtoSet::from_str("2-5,8")?;
    ///
    /// assert!(protoset.contains(5));
    /// assert!(!protoset.contains(10));
    ///
    /// // We can also equivalently call `ProtoSet::from_str` by doing (all
    /// // implementations of `FromStr` can be called this way, this one isn't
    /// // special):
    /// let protoset: ProtoSet = "4-6,12".parse()?;
    ///
    /// // Calling it (either way) can take really large ranges (up to `u32::MAX`):
    /// let protoset: ProtoSet = "1-70000".parse()?;
    /// let protoset: ProtoSet = "1-4294967296".parse()?;
    ///
    /// // There are lots of ways to get an `Err` from this function.  Here are
    /// // a few:
    /// assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("="));
    /// assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("-"));
    /// assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("not_an_int"));
    /// assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("3-"));
    /// assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("1-,4"));
    ///
    /// // Things which would get parsed into an _empty_ `ProtoSet` are,
    /// // however, legal, and result in an empty `ProtoSet`:
    /// assert_eq!(Ok(ProtoSet::default()), ProtoSet::from_str(""));
    /// assert_eq!(Ok(ProtoSet::default()), ProtoSet::from_str(",,,"));
    /// #
    /// # Ok(protoset)
    /// # }
    /// # fn main() { do_test(); }  // wrap the test so we can use the ? operator
    /// ```
    fn from_str(version_string: &str) -> Result<Self, Self::Err> {
        let mut pairs: Vec<(Version, Version)> = Vec::new();
        let pieces: ::std::str::Split<char> = version_string.trim().split(',');

        for piece in pieces {
            let p: &str = piece.trim();

            if p.is_empty() {
                continue;
            } else if p.contains('-') {
                let mut pair = p.splitn(2, '-');

                let low = pair.next().ok_or(ProtoverError::Unparseable)?;
                let high = pair.next().ok_or(ProtoverError::Unparseable)?;

                let lo: Version = low.parse().or(Err(ProtoverError::Unparseable))?;
                let hi: Version = high.parse().or(Err(ProtoverError::Unparseable))?;

                if lo == u32::MAX || hi == u32::MAX {
                    return Err(ProtoverError::ExceedsMax);
                }
                pairs.push((lo, hi));
            } else {
                let v: u32 = p.parse().or(Err(ProtoverError::Unparseable))?;

                if v == u32::MAX {
                    return Err(ProtoverError::ExceedsMax);
                }
                pairs.push((v, v));
            }
        }
        // If we were passed in an empty string, or a bunch of whitespace, or
        // simply a comma, or a pile of commas, then return an empty ProtoSet.
        if pairs.is_empty() {
            return Ok(ProtoSet::default());
        }
        pairs.sort_unstable();
        pairs.dedup();
        ProtoSet::from_sorted(pairs)
    }
}

impl ToString for ProtoSet {
    /// Contracts a `ProtoSet` of versions into a string.
    ///
    /// # Returns
    ///
    /// A `String` representation of this `ProtoSet` in ascending order.
    fn to_string(&self) -> String {
        let mut final_output: Vec<String> = Vec::new();

        for &(lo, hi) in self.iter() {
            if lo != hi {
                debug_assert!(lo < hi);
                final_output.push(format!("{}-{}", lo, hi));
            } else {
                final_output.push(format!("{}", lo));
            }
        }
        final_output.join(",")
    }
}

/// Checks to see if there is a continuous range of integers, starting at the
/// first in the list. Returns the range of integers.
///
/// # Inputs
///
/// `list`, an ordered slice of `Version` integers representing the
/// supported versions for a single protocol.
///
/// # Returns
///
/// An `Option` containing a tuple consisting of the first `Version` in the
/// slice, and the `Version` of the last integer in the range. The two will
/// be equal if no larger range is found. Returns None if passed an empty slice.
///
/// For example, if given &[1, 2, 3, 5], find_range will return Some((1, 3)),
/// as there is a continuous range of 1 to 3.
fn find_range(list: &[Version]) -> Option<(Version, Version)> {
    let mut iter = list.iter();
    let first = match iter.next() {
        Some(&n) => n,
        None => return None,
    };
    let last = iter.fold(first, |latest, &n| if latest + 1 == n { n } else { latest });
    Some((first, last))
}

impl From<Vec<Version>> for ProtoSet {
    fn from(mut v: Vec<Version>) -> ProtoSet {
        let mut version_pairs: Vec<(Version, Version)> = Vec::new();

        v.sort_unstable();
        v.dedup();
        let mut v = &v[..];

        while let Some((first, last)) = find_range(v) {
            let index = (last - first) as usize;
            assert_eq!(last, v[index]);

            version_pairs.push((first, last));
            v = &v[index + 1..];
        }
        ProtoSet::from_sorted(version_pairs).unwrap_or_default()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_range() {
        assert_eq!(None, find_range(&vec![]));
        assert_eq!(Some((1, 1)), find_range(&vec![1]));
        assert_eq!(Some((1, 2)), find_range(&vec![1, 2]));
        assert_eq!(Some((1, 3)), find_range(&vec![1, 2, 3]));
        assert_eq!(Some((1, 3)), find_range(&vec![1, 2, 3, 5]));
    }

    macro_rules! assert_contains_each {
        ($protoset:expr, $versions:expr) => {
            for &version in $versions {
                assert!($protoset.contains(version));
            }
        };
    }

    macro_rules! test_protoset_contains_versions {
        ($list:expr, $str:expr) => {
            let versions: &[Version] = $list;
            let protoset: Result<ProtoSet, ProtoverError> = ProtoSet::from_str($str);

            assert!(protoset.is_ok());
            let p = protoset.unwrap();
            assert_contains_each!(p, versions);
        };
    }

    #[test]
    fn test_versions_from_str() {
        test_protoset_contains_versions!(&[], "");
        test_protoset_contains_versions!(&[1], "1");
        test_protoset_contains_versions!(&[1, 2], "1,2");
        test_protoset_contains_versions!(&[1, 2, 3], "1-3");
        test_protoset_contains_versions!(&[0, 1], "0-1");
        test_protoset_contains_versions!(&[1, 2, 5], "1-2,5");
        test_protoset_contains_versions!(&[1, 3, 4, 5], "1,3-5");
        test_protoset_contains_versions!(&[42, 55, 56, 57, 58], "42,55-58");
        test_protoset_contains_versions!(&[0, 4294967294], "0-4294967294");
    }

    #[test]
    fn test_versions_from_str_ab() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("a,b"));
    }

    #[test]
    fn test_versions_from_str_negative_1() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("-1"));
    }

    #[test]
    fn test_versions_from_str_hyphens() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("--1"));
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("-1-2"));
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("1--2"));
    }

    #[test]
    fn test_versions_from_str_triple() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("1-2-3"));
    }

    #[test]
    fn test_versions_from_str_1exclam() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("1,!"));
    }

    #[test]
    fn test_versions_from_str_percent_equal() {
        assert_eq!(Err(ProtoverError::Unparseable), ProtoSet::from_str("%="));
    }

    #[test]
    fn test_versions_from_str_overlap() {
        assert_eq!(Err(ProtoverError::Overlap), ProtoSet::from_str("1-3,2-4"));
    }

    #[test]
    fn test_versions_from_slice_overlap() {
        assert_eq!(
            Err(ProtoverError::Overlap),
            ProtoSet::from_slice(&[(1, 3), (2, 4)])
        );
    }

    #[test]
    fn test_versions_from_str_max() {
        assert_eq!(
            Err(ProtoverError::ExceedsMax),
            ProtoSet::from_str("4294967295")
        );
    }

    #[test]
    fn test_versions_from_slice_max() {
        assert_eq!(
            Err(ProtoverError::ExceedsMax),
            ProtoSet::from_slice(&[(4294967295, 4294967295)])
        );
    }

    #[test]
    fn test_versions_from_str_maxplusone() {
        assert_eq!(
            Err(ProtoverError::Unparseable),
            ProtoSet::from_str("4294967296")
        );
    }

    #[test]
    fn test_protoset_contains() {
        let protoset: ProtoSet = ProtoSet::from_slice(&[(0, 5), (7, 9), (13, 14)]).unwrap();

        for x in 0..6 {
            assert!(protoset.contains(x), format!("should contain {}", x));
        }
        for x in 7..10 {
            assert!(protoset.contains(x), format!("should contain {}", x));
        }
        for x in 13..15 {
            assert!(protoset.contains(x), format!("should contain {}", x));
        }

        for &x in &[6, 10, 11, 12, 15, 42, 43, 44, 45, 1234584] {
            assert!(!protoset.contains(x), format!("should not contain {}", x));
        }
    }

    #[test]
    fn test_protoset_contains_0_3() {
        let protoset: ProtoSet = ProtoSet::from_slice(&[(0, 3)]).unwrap();

        for x in 0..4 {
            assert!(protoset.contains(x), format!("should contain {}", x));
        }
    }

    macro_rules! assert_protoset_from_vec_contains_all {
        ($($x:expr),*) => (
            let vec: Vec<Version> = vec!($($x),*);
            let protoset: ProtoSet = vec.clone().into();

            for &x in &vec {
                assert!(protoset.contains(x));
            }
        )
    }

    #[test]
    fn test_protoset_from_vec_123() {
        assert_protoset_from_vec_contains_all!(1, 2, 3);
    }

    #[test]
    fn test_protoset_from_vec_0_315() {
        assert_protoset_from_vec_contains_all!(0, 1, 2, 3, 15);
    }

    #[test]
    fn test_protoset_from_vec_unordered() {
        let v: Vec<Version> = vec![2, 3, 8, 4, 3, 9, 7, 2];
        let ps: ProtoSet = v.into();

        assert_eq!(ps.to_string(), "2-4,7-9");
    }

    #[test]
    fn test_protoset_into_vec() {
        let ps: ProtoSet = "1-13,42,9001,4294967294".parse().unwrap();
        let v: Vec<Version> = ps.expand().collect();

        assert!(v.contains(&7));
        assert!(v.contains(&9001));
        assert!(v.contains(&4294967294));
    }
}

#[cfg(all(test, feature = "bench"))]
mod bench {
    use super::*;
}
