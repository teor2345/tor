// Copyright (c) 2016-2018, The Tor Project, Inc. */
// See LICENSE for licensing information */

use std::collections::hash_map;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fmt;
use std::str;
use std::str::FromStr;
use std::string::String;

use external::c_tor_version_as_new_as;

use errors::ProtoverError;
use protoset::ProtoSet;
use protoset::Version;

/// The first version of Tor that included "proto" entries in its descriptors.
/// Authorities should use this to decide whether to guess proto lines.
///
/// C_RUST_COUPLED:
///     protover.h `FIRST_TOR_VERSION_TO_ADVERTISE_PROTOCOLS`
const FIRST_TOR_VERSION_TO_ADVERTISE_PROTOCOLS: &str = "0.2.9.3-alpha";

/// The maximum number of subprotocol version numbers we will attempt to expand
/// before concluding that someone is trying to DoS us
///
/// C_RUST_COUPLED: protover.c `MAX_PROTOCOLS_TO_EXPAND`
const MAX_PROTOCOLS_TO_EXPAND: usize = (1 << 16);

/// The maximum size an `UnknownProtocol`'s name may be.
pub(crate) const MAX_PROTOCOL_NAME_LENGTH: usize = 100;

/// Known subprotocols in Tor. Indicates which subprotocol a relay supports.
///
/// C_RUST_COUPLED: protover.h `protocol_type_t`
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum Protocol {
    Cons,
    Desc,
    DirCache,
    HSDir,
    HSIntro,
    HSRend,
    Link,
    LinkAuth,
    Microdesc,
    Relay,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Translates a string representation of a protocol into a Proto type.
/// Error if the string is an unrecognized protocol name.
///
/// C_RUST_COUPLED: protover.c `PROTOCOL_NAMES`
impl FromStr for Protocol {
    type Err = ProtoverError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Cons" => Ok(Protocol::Cons),
            "Desc" => Ok(Protocol::Desc),
            "DirCache" => Ok(Protocol::DirCache),
            "HSDir" => Ok(Protocol::HSDir),
            "HSIntro" => Ok(Protocol::HSIntro),
            "HSRend" => Ok(Protocol::HSRend),
            "Link" => Ok(Protocol::Link),
            "LinkAuth" => Ok(Protocol::LinkAuth),
            "Microdesc" => Ok(Protocol::Microdesc),
            "Relay" => Ok(Protocol::Relay),
            _ => Err(ProtoverError::UnknownProtocol),
        }
    }
}

/// A protocol string which is not one of the `Protocols` we currently know
/// about.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct UnknownProtocol(String);

impl fmt::Display for UnknownProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for UnknownProtocol {
    type Err = ProtoverError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(ProtoverError::Unparseable)
        } else if s.len() <= MAX_PROTOCOL_NAME_LENGTH {
            Ok(UnknownProtocol(s.to_string()))
        } else {
            Err(ProtoverError::ExceedsNameLimit)
        }
    }
}

impl UnknownProtocol {
    /// Create an `UnknownProtocol`, ignoring whether or not it
    /// exceeds MAX_PROTOCOL_NAME_LENGTH.
    fn from_str_any_len(s: &str) -> Result<Self, ProtoverError> {
        if s.is_empty() {
            Err(ProtoverError::Unparseable)
        } else {
            Ok(UnknownProtocol(s.to_string()))
        }
    }
}

impl From<Protocol> for UnknownProtocol {
    fn from(p: Protocol) -> UnknownProtocol {
        UnknownProtocol(p.to_string())
    }
}

/// Get a CStr representation of current supported protocols, for
/// passing to C, or for converting to a `&str` for Rust.
///
/// # Returns
///
/// An `&'static CStr` whose value is the existing protocols supported by tor.
/// Returned data is in the format as follows:
///
/// "HSDir=1-1 LinkAuth=1"
///
/// # Note
///
/// Rust code can use the `&'static CStr` as a normal `&'a str` by
/// calling `protover::get_supported_protocols`.
///
//  C_RUST_COUPLED: protover.c `protover_get_supported_protocols`
pub(crate) fn get_supported_protocols_cstr() -> &'static CStr {
    cstr!(
        "Cons=1-2 \
         Desc=1-2 \
         DirCache=1-2 \
         HSDir=1-2 \
         HSIntro=3-4 \
         HSRend=1-2 \
         Link=1-5 \
         LinkAuth=1,3 \
         Microdesc=1-2 \
         Relay=1-2"
    )
}

/// A map of protocol names to the versions of them which are supported.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct ProtoEntry(HashMap<Protocol, ProtoSet>);

impl ProtoEntry {
    /// Get an iterator over the `Protocol`s and their `ProtoSet`s in this `ProtoEntry`.
    pub fn iter(&self) -> hash_map::Iter<Protocol, ProtoSet> {
        self.0.iter()
    }

    /// Translate the supported tor versions from a string into a
    /// ProtoEntry, which is useful when looking up a specific
    /// subprotocol.
    pub fn supported() -> Result<Self, ProtoverError> {
        let supported_cstr: &'static CStr = get_supported_protocols_cstr();
        let supported: &str = supported_cstr.to_str().unwrap_or("");

        supported.parse()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, protocol: &Protocol) -> Option<&ProtoSet> {
        self.0.get(protocol)
    }

    pub fn insert(&mut self, key: Protocol, value: ProtoSet) {
        self.0.insert(key, value);
    }

    pub fn remove(&mut self, key: &Protocol) -> Option<ProtoSet> {
        self.0.remove(key)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for ProtoEntry {
    type Err = ProtoverError;

    /// Parse a string of subprotocol types and their version numbers.
    ///
    /// # Inputs
    ///
    /// * A `protocol_entry` string, comprised of a keywords, an "=" sign, and
    /// one or more version numbers, each separated by a space.  For example,
    /// `"Cons=3-4 HSDir=1"`.
    ///
    /// # Returns
    ///
    /// A `Result` whose `Ok` value is a `ProtoEntry`.
    /// Otherwise, the `Err` value of this `Result` is a `ProtoverError`.
    fn from_str(protocol_entry: &str) -> Result<Self, Self::Err> {
        let mut parsed = Self::default();
        let parts = UnvalidatedProtoEntry::parse_protocol_and_version_str(protocol_entry);

        let parse_parts = |(len, name, versions): (usize, &str, ProtoSet)| {
            let protocol: Protocol = name.parse()?;

            if len > MAX_PROTOCOLS_TO_EXPAND {
                Err(ProtoverError::ExceedsMax)
            } else {
                Ok((protocol, versions))
            }
        };

        let count_and_parse = |(i, r): (usize, Result<_, _>)| {
            r.map(|(name, vers)| (i + 1, name, vers))
                .and_then(parse_parts)
        };
        parsed.0 = try!(parts.enumerate().map(count_and_parse).collect());
        Ok(parsed)
    }
}

/// Generate an implementation of `ToString` for either a `ProtoEntry` or an
/// `UnvalidatedProtoEntry`.
macro_rules! impl_to_string_for_proto_entry {
    ($t:ty) => {
        impl ToString for $t {
            fn to_string(&self) -> String {
                let mut parts: Vec<String> = Vec::new();

                for (protocol, versions) in self.iter() {
                    parts.push(format!("{}={}", protocol.to_string(), versions.to_string()));
                }
                parts.sort_unstable();
                parts.join(" ")
            }
        }
    };
}

impl_to_string_for_proto_entry!(ProtoEntry);
impl_to_string_for_proto_entry!(UnvalidatedProtoEntry);

/// A `ProtoEntry`, but whose `Protocols` can be any `UnknownProtocol`, not just
/// the supported ones enumerated in `Protocols`.  The protocol versions are
/// validated, however.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct UnvalidatedProtoEntry(HashMap<UnknownProtocol, ProtoSet>);

impl UnvalidatedProtoEntry {
    /// Get an iterator over the `Protocol`s and their `ProtoSet`s in this `ProtoEntry`.
    pub fn iter(&self) -> hash_map::Iter<UnknownProtocol, ProtoSet> {
        self.0.iter()
    }

    pub fn get(&self, protocol: &UnknownProtocol) -> Option<&ProtoSet> {
        self.0.get(protocol)
    }

    pub fn insert(&mut self, key: UnknownProtocol, value: ProtoSet) {
        self.0.insert(key, value);
    }

    pub fn remove(&mut self, key: &UnknownProtocol) -> Option<ProtoSet> {
        self.0.remove(key)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.iter().map(|(_, versions)| versions.len()).sum()
    }

    /// Determine if we support every protocol a client supports, and if not,
    /// determine which protocols we do not have support for.
    ///
    /// # Returns
    ///
    /// Return parameters which the client supports but which we do not.
    ///
    /// # Examples
    /// ```
    /// use protover::UnvalidatedProtoEntry;
    ///
    /// let protocols: UnvalidatedProtoEntry = "LinkAuth=1 Microdesc=1-2 Relay=2".parse().unwrap();
    /// let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
    /// assert_eq!(true, unsupported.is_empty());
    ///
    /// let protocols: UnvalidatedProtoEntry = "Link=1-2 Wombat=9".parse().unwrap();
    /// let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
    /// assert_eq!(false, unsupported.is_empty());
    /// assert_eq!("Wombat=9", &unsupported.to_string());
    /// ```
    pub fn get_unsupported(&self) -> Self {
        let mut unsupported = Self::default();
        let supported: ProtoEntry = match ProtoEntry::supported() {
            Ok(x) => x,
            Err(_) => return self.clone(),
        };

        let emptyset = ProtoSet::default();
        let f = |(protocol, versions): (&UnknownProtocol, &ProtoSet)| {
            let is_supported: Result<Protocol, ProtoverError> = protocol.0.parse();

            // If the protocol wasn't in the enum, or wasn't in the map, then we
            // don't support any of its versions.
            let supported_versions: &ProtoSet = is_supported
                .ok()
                .and_then(|supported_protocol| supported.get(&supported_protocol))
                .unwrap_or(&emptyset);

            let unsupported_versions = versions.and_not_in(supported_versions);

            if unsupported_versions.is_empty() {
                None
            } else {
                Some((protocol.clone(), unsupported_versions))
            }
        };
        unsupported.0 = self.iter().filter_map(f).collect();

        unsupported
    }

    /// Determine if we have support for some protocol and version.
    ///
    /// # Inputs
    ///
    /// * `proto`, an `UnknownProtocol` to test support for
    /// * `vers`, a `Version` which we will go on to determine whether the
    /// specified protocol supports.
    ///
    /// # Return
    ///
    /// Returns `true` iff this `UnvalidatedProtoEntry` includes support for the
    /// indicated protocol and version, and `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::str::FromStr;
    /// use protover::*;
    /// # use protover::errors::ProtoverError;
    ///
    /// # fn do_test () -> Result<UnvalidatedProtoEntry, ProtoverError> {
    /// let proto: UnvalidatedProtoEntry = "Link=3-4 Cons=1 Doggo=3-5".parse()?;
    /// assert_eq!(true, proto.supports_protocol(&Protocol::Cons.into(), 1));
    /// assert_eq!(false, proto.supports_protocol(&Protocol::Cons.into(), 5));
    /// assert_eq!(true, proto.supports_protocol(&UnknownProtocol::from_str("Doggo")?, 4));
    /// # Ok(proto)
    /// # } fn main () { do_test(); }
    /// ```
    pub fn supports_protocol(&self, proto: &UnknownProtocol, vers: Version) -> bool {
        let supported_versions: &ProtoSet = match self.get(proto) {
            Some(n) => n,
            None => return false,
        };
        supported_versions.contains(vers)
    }

    /// As `UnvalidatedProtoEntry::supports_protocol()`, but also returns `true`
    /// if any later version of the protocol is supported.
    ///
    /// # Examples
    /// ```
    /// use protover::*;
    /// # use protover::errors::ProtoverError;
    ///
    /// # fn do_test () -> Result<UnvalidatedProtoEntry, ProtoverError> {
    /// let proto: UnvalidatedProtoEntry = "Link=3-4 Cons=5".parse()?;
    ///
    /// assert_eq!(true, proto.supports_protocol_or_later(&Protocol::Cons.into(), 5));
    /// assert_eq!(true, proto.supports_protocol_or_later(&Protocol::Cons.into(), 4));
    /// assert_eq!(false, proto.supports_protocol_or_later(&Protocol::Cons.into(), 6));
    /// # Ok(proto)
    /// # } fn main () { do_test(); }
    /// ```
    pub fn supports_protocol_or_later(&self, proto: &UnknownProtocol, vers: Version) -> bool {
        let supported_versions: &ProtoSet = match self.get(&proto) {
            Some(n) => n,
            None => return false,
        };
        supported_versions.iter().any(|&v| v.1 >= vers)
    }

    /// Split a string containing (potentially) several protocols and their
    /// versions into a sequence of tuples each consisting of a string protocol
    /// name and its corresponding parsed ProtoSet.
    ///
    /// # Inputs
    ///
    /// A &str in the form `"Link=3-4 Cons=5"`.
    ///
    /// # Returns
    ///
    /// An iterator of `Result` values whose `Ok` variant is a `(&str, ProtoSet)`
    /// of `(protocol, versions)`, or whose `Err` variant is a `ProtoverError`.
    ///
    /// # Errors
    ///
    /// This will error with a `ProtoverError::Unparseable` if any of the
    /// following are true:
    ///
    /// * If an entry has no equals sign, e.g. `"Cons=1,3 Desc"`.
    /// * If there is leading or trailing whitespace, e.g. `" Cons=1,3 Link=3"`.
    /// * If there is any other extra whitespice, e.g. `"Cons=1,3  Link=3"`.
    /// * If a version string is malformed. See `impl FromStr for ProtoSet`.
    fn parse_protocol_and_version_str<'a>(
        protocol_entry: &'a str,
    ) -> impl Iterator<Item = Result<(&'a str, ProtoSet), ProtoverError>> {
        let parse_subproto = |subproto: &'a str| {
            let mut parts = subproto.splitn(2, '=');

            let name = parts.next().ok_or(ProtoverError::Unparseable)?;
            let vers = parts.next().ok_or(ProtoverError::Unparseable)?;
            Ok((name, vers.parse()?))
        };
        protocol_entry.split(' ').map(parse_subproto)
    }
}

impl FromStr for UnvalidatedProtoEntry {
    type Err = ProtoverError;

    /// Parses a protocol list without validating the protocol names.
    ///
    /// # Inputs
    ///
    /// * `protocol_entry`, a string comprised of keys and values, both which are
    /// strings. The keys are the protocol names while values are a string
    /// representation of the supported versions.
    ///
    /// The input is _not_ expected to be a subset of the Protocol types
    ///
    /// # Returns
    ///
    /// A `Result` whose `Ok` value is an `UnvalidatedProtoEntry`.
    ///
    /// The returned `Result`'s `Err` value is an `ProtoverError`.
    ///
    /// # Errors
    ///
    /// This function will error if:
    ///
    /// * The protocol string does not follow the "protocol_name=version_list"
    ///   expected format, or
    /// * If the version string is malformed. See `impl FromStr for ProtoSet`.
    fn from_str(protocol_entry: &str) -> Result<Self, Self::Err> {
        let mut parsed = Self::default();
        let parts = UnvalidatedProtoEntry::parse_protocol_and_version_str(protocol_entry);

        let parse_parts = |(name, versions): (&str, ProtoSet)| {
            let protocol = UnknownProtocol::from_str(name)?;

            Ok((protocol, versions))
        };
        parsed.0 = try!(parts.map(|r| r.and_then(parse_parts)).collect());
        Ok(parsed)
    }
}

impl UnvalidatedProtoEntry {
    /// Create an `UnknownProtocol`, ignoring whether or not it
    /// exceeds MAX_PROTOCOL_NAME_LENGTH.
    pub(crate) fn from_str_any_len(protocol_entry: &str) -> Result<Self, ProtoverError> {
        let mut parsed = Self::default();
        let parts = UnvalidatedProtoEntry::parse_protocol_and_version_str(protocol_entry);

        let parse_parts = |(name, versions): (&str, ProtoSet)| {
            let protocol = UnknownProtocol::from_str_any_len(name)?;

            Ok((protocol, versions))
        };
        parsed.0 = try!(parts.map(|r| r.and_then(parse_parts)).collect());
        Ok(parsed)
    }
}

/// Pretend a `ProtoEntry` is actually an `UnvalidatedProtoEntry`.
impl From<ProtoEntry> for UnvalidatedProtoEntry {
    fn from(proto_entry: ProtoEntry) -> UnvalidatedProtoEntry {
        let mut unvalidated = Self::default();
        unvalidated.0 = proto_entry
            .0
            .into_iter()
            .map(|(protocol, versions)| (UnknownProtocol::from(protocol), versions))
            .collect();
        unvalidated
    }
}

/// A mapping of protocols to a count of how many times each of their `Version`s
/// were voted for or supported.
///
/// # Warning
///
/// The "protocols" are *not* guaranteed to be known/supported `Protocol`s, in
/// order to allow new subprotocols to be introduced even if Directory
/// Authorities don't yet know of them.
#[derive(Default)]
pub struct ProtoverVote(HashMap<UnknownProtocol, HashMap<Version, usize>>);

impl IntoIterator for ProtoverVote {
    type Item = (UnknownProtocol, HashMap<Version, usize>);
    type IntoIter = hash_map::IntoIter<UnknownProtocol, HashMap<Version, usize>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ProtoverVote {
    pub fn entry(
        &mut self,
        key: UnknownProtocol,
    ) -> hash_map::Entry<UnknownProtocol, HashMap<Version, usize>> {
        self.0.entry(key)
    }

    /// Protocol voting implementation.
    ///
    /// Given a slice of `UnvalidatedProtoEntry`s and a vote `threshold`, return
    /// a new `UnvalidatedProtoEntry` encoding all of the protocols that are
    /// listed by at least `threshold` of the inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use protover::ProtoverVote;
    /// use protover::UnvalidatedProtoEntry;
    ///
    /// let protos: &[UnvalidatedProtoEntry] = &["Link=3-4".parse().unwrap(),
    ///                                          "Link=3".parse().unwrap()];
    /// let vote = ProtoverVote::compute(protos, 2);
    /// assert_eq!("Link=3", vote.to_string());
    /// ```
    // C_RUST_COUPLED: protover.c protover_compute_vote
    pub fn compute(
        proto_entries: &[UnvalidatedProtoEntry],
        threshold: usize,
    ) -> UnvalidatedProtoEntry {
        let mut all_count = Self::default();
        let mut final_output: UnvalidatedProtoEntry = Default::default();

        if proto_entries.is_empty() {
            return final_output;
        }

        // parse and collect all of the protos and their versions and collect them
        for vote in proto_entries {
            // C_RUST_DIFFERS: This doesn't actually differ, bu this check on
            // the total is here to make it match.  Because the C version calls
            // expand_protocol_list() which checks if there would be too many
            // subprotocols *or* individual version numbers, i.e. more than
            // MAX_PROTOCOLS_TO_EXPAND, and does this *per vote*, we need to
            // match it's behaviour and ensure we're not allowing more than it
            // would.
            if vote.len() > MAX_PROTOCOLS_TO_EXPAND {
                continue;
            }

            for (protocol, versions) in vote.iter() {
                let supported_vers: &mut HashMap<Version, usize> =
                    all_count.entry(protocol.clone()).or_insert(HashMap::new());

                for version in versions.expand() {
                    let counter: &mut usize = supported_vers.entry(version).or_insert(0);
                    *counter += 1;
                }
            }
        }

        for (protocol, mut versions) in all_count {
            // Go through and remove versions that are less than the threshold
            versions.retain(|_, &mut count| count as usize >= threshold);

            if !versions.is_empty() {
                let voted_versions: Vec<Version> = versions.keys().cloned().collect();
                let voted_protoset: ProtoSet = ProtoSet::from(voted_versions);

                final_output.insert(protocol, voted_protoset);
            }
        }
        final_output
    }
}

/// Returns a boolean indicating whether the given protocol and version is
/// supported in any of the existing Tor protocols
///
/// # Examples
/// ```
/// use protover::is_supported_here;
/// use protover::Protocol;
///
/// let is_supported = is_supported_here(&Protocol::Link, 10);
/// assert_eq!(false, is_supported);
///
/// let is_supported = is_supported_here(&Protocol::Link, 1);
/// assert_eq!(true, is_supported);
/// ```
pub fn is_supported_here(proto: &Protocol, vers: Version) -> bool {
    let currently_supported: ProtoEntry = match ProtoEntry::supported() {
        Ok(result) => result,
        Err(_) => return false,
    };
    let supported_versions = match currently_supported.get(proto) {
        Some(n) => n,
        None => return false,
    };
    supported_versions.contains(vers)
}

/// Since older versions of Tor cannot infer their own subprotocols,
/// determine which subprotocols are supported by older Tor versions.
///
/// # Inputs
///
/// * `version`, a string comprised of "[0-9a-z.-]"
///
/// # Returns
///
/// A `&'static CStr` encoding a list of protocol names and supported
/// versions. The string takes the following format:
///
/// "HSDir=1-1 LinkAuth=1"
///
/// This function returns the protocols that are supported by the version input,
/// only for tor versions older than `FIRST_TOR_VERSION_TO_ADVERTISE_PROTOCOLS`
/// (but not older than 0.2.4.19).  For newer tors (or older than 0.2.4.19), it
/// returns an empty string.
///
/// # Note
///
/// This function is meant to be called for/within FFI code.  If you'd
/// like to use this code in Rust, please see `compute_for_old_tor()`.
//
// C_RUST_COUPLED: src/rust/protover.c `compute_for_old_tor`
pub(crate) fn compute_for_old_tor_cstr(version: &str) -> &'static CStr {
    let empty: &'static CStr = cstr!("");

    if c_tor_version_as_new_as(version, FIRST_TOR_VERSION_TO_ADVERTISE_PROTOCOLS) {
        return empty;
    }
    if c_tor_version_as_new_as(version, "0.2.9.1-alpha") {
        return cstr!(
            "Cons=1-2 Desc=1-2 DirCache=1 HSDir=1 HSIntro=3 HSRend=1-2 \
             Link=1-4 LinkAuth=1 Microdesc=1-2 Relay=1-2"
        );
    }
    if c_tor_version_as_new_as(version, "0.2.7.5") {
        return cstr!(
            "Cons=1-2 Desc=1-2 DirCache=1 HSDir=1 HSIntro=3 HSRend=1 \
             Link=1-4 LinkAuth=1 Microdesc=1-2 Relay=1-2"
        );
    }
    if c_tor_version_as_new_as(version, "0.2.4.19") {
        return cstr!(
            "Cons=1 Desc=1 DirCache=1 HSDir=1 HSIntro=3 HSRend=1 \
             Link=1-4 LinkAuth=1 Microdesc=1 Relay=1-2"
        );
    }
    empty
}

/// Since older versions of Tor cannot infer their own subprotocols,
/// determine which subprotocols are supported by older Tor versions.
///
/// # Inputs
///
/// * `version`, a string comprised of "[0-9a-z.-]"
///
/// # Returns
///
/// A `Result` whose `Ok` value is an `&'static str` encoding a list of protocol
/// names and supported versions. The string takes the following format:
///
/// "HSDir=1-1 LinkAuth=1"
///
/// This function returns the protocols that are supported by the version input,
/// only for tor versions older than `FIRST_TOR_VERSION_TO_ADVERTISE_PROTOCOLS`.
/// (but not older than 0.2.4.19).  For newer tors (or older than 0.2.4.19), its
/// `Ok` `Result` contains an empty string.
///
/// Otherwise, its `Err` contains a `ProtoverError::Unparseable` if the
/// `version` string was invalid utf-8.
///
/// # Note
///
/// This function is meant to be called for/within non-FFI Rust code.
//
// C_RUST_COUPLED: src/rust/protover.c `compute_for_old_tor`
pub fn compute_for_old_tor(version: &str) -> Result<&'static str, ProtoverError> {
    // .to_str() fails with a Utf8Error if it couldn't validate the
    // utf-8, so convert that here into an Unparseable ProtoverError.
    compute_for_old_tor_cstr(version)
        .to_str()
        .or(Err(ProtoverError::Unparseable))
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use std::string::ToString;

    use super::*;

    macro_rules! assert_protoentry_is_parseable {
        ($e:expr) => {
            let protoentry: Result<ProtoEntry, ProtoverError> = $e.parse();

            assert!(protoentry.is_ok(), format!("{:?}", protoentry.err()));
        };
    }

    macro_rules! assert_protoentry_is_unparseable {
        ($e:expr) => {
            let protoentry: Result<ProtoEntry, ProtoverError> = $e.parse();

            assert!(protoentry.is_err());
        };
    }

    #[test]
    fn test_protoentry_from_str_multiple_protocols_multiple_versions() {
        assert_protoentry_is_parseable!("Cons=3-4 Link=1,3-5");
    }

    #[test]
    fn test_protoentry_from_str_empty() {
        assert_protoentry_is_unparseable!("");
        assert_protoentry_is_unparseable!("=");
        assert_protoentry_is_unparseable!("=1-2");

        let unvalidated: Result<UnvalidatedProtoEntry, ProtoverError> = "=1-2".parse();
        assert_eq!(unvalidated, Err(ProtoverError::Unparseable));
    }

    #[test]
    fn test_protoentry_from_str_single_protocol_single_version() {
        assert_protoentry_is_parseable!("HSDir=1");
    }

    #[test]
    fn test_protoentry_from_str_unknown_protocol() {
        assert_protoentry_is_unparseable!("Ducks=5-7,8");
    }

    #[test]
    fn test_protoentry_from_str_allowed_number_of_versions() {
        assert_protoentry_is_parseable!("Desc=1-4294967294");
    }

    #[test]
    fn test_protoentry_from_str_too_many_versions() {
        assert_protoentry_is_unparseable!("Desc=1-4294967295");
    }

    #[test]
    fn test_protoentry_all_supported_single_protocol_single_version() {
        let protocol: UnvalidatedProtoEntry = "Cons=1".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocol.get_unsupported();
        assert_eq!(true, unsupported.is_empty());
    }

    #[test]
    fn test_protoentry_all_supported_multiple_protocol_multiple_versions() {
        let protocols: UnvalidatedProtoEntry = "Link=3-4 Desc=2".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
        assert_eq!(true, unsupported.is_empty());
    }

    #[test]
    fn test_protoentry_all_supported_three_values() {
        let protocols: UnvalidatedProtoEntry = "LinkAuth=1 Microdesc=1-2 Relay=2".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
        assert_eq!(true, unsupported.is_empty());
    }

    #[test]
    fn test_protoentry_all_supported_unknown_protocol() {
        let protocols: UnvalidatedProtoEntry = "Wombat=9".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
        assert_eq!(false, unsupported.is_empty());
        assert_eq!("Wombat=9", &unsupported.to_string());
    }

    #[test]
    fn test_protoentry_all_supported_unsupported_high_version() {
        let protocols: UnvalidatedProtoEntry = "HSDir=12-100".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
        assert_eq!(false, unsupported.is_empty());
        assert_eq!("HSDir=12-100", &unsupported.to_string());
    }

    #[test]
    fn test_protoentry_all_supported_unsupported_low_version() {
        let protocols: UnvalidatedProtoEntry = "Cons=0-1".parse().unwrap();
        let unsupported: UnvalidatedProtoEntry = protocols.get_unsupported();
        assert_eq!(false, unsupported.is_empty());
        assert_eq!("Cons=0", &unsupported.to_string());
    }

    #[test]
    fn test_contract_protocol_list() {
        let mut versions = "";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1-2";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1,3";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1-4";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1,3,5-7";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );

        versions = "1-3,500";
        assert_eq!(
            String::from(versions),
            ProtoSet::from_str(&versions).unwrap().to_string()
        );
    }
}
