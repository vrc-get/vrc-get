//! The semantic versioning used by vpm.
//!
//! Since vpm uses [semver.net], the semver notation is based on npm, which is different from the semver crate.
//! So, vrc-get-vpm currently uses its own versioning system.
//!
//! [semver.net]: https://github.com/adamreeve/semver.net

pub use range::DependencyRange;
pub use range::PrereleaseAcceptance;
pub use range::VersionRange;
use std::fmt::Debug;
pub use unity_version::ReleaseType;
pub use unity_version::UnityVersion;
pub use version::StrictEqVersion;
pub use version::Version;

macro_rules! from_str_impl {
    ($ty: ty) => {
        impl FromStr for $ty {
            type Err = ParseVersionError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut buffer = ParsingBuf::new(s);
                let result = FromParsingBuf::parse(&mut buffer)?;
                if buffer.first().is_some() {
                    return Err(ParseVersionError::invalid());
                }
                Ok(result)
            }
        }
    };
}

macro_rules! serialize_to_string {
    ($ty: ty) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(&std::string::ToString::to_string(self))
            }
        }
    };
}

macro_rules! deserialize_from_str {
    ($ty: ty, $name: literal) => {
        impl<'de> ::serde::de::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::de::Deserializer<'de>,
            {
                struct Visitor;
                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $ty;

                    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                        formatter.write_str($name)
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        std::str::FromStr::from_str(v).map_err(E::custom)
                    }
                }
                deserializer.deserialize_str(Visitor)
            }
        }
    };
}

mod actual_identifier;
mod identifier;
mod parsing_buf;
mod range;
mod segment;
mod unity_version;
#[allow(clippy::module_inception)]
mod version;

use segment::Segment;

pub use actual_identifier::BuildMetadata;
pub use actual_identifier::Prerelease;
use parsing_buf::FromParsingBuf;
use parsing_buf::ParseVersionError;
use parsing_buf::ParsingBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_cmp() {
        fn test(greater: &str, lesser: &str) {
            let greater = Version::from_str(greater).expect(greater);
            let lesser = Version::from_str(lesser).expect(lesser);
            assert!(greater > lesser, "{greater} > {lesser}");
        }
        // test set are from node-semver
        // Copyright (c) Isaac Z. Schlueter and Contributors
        // Originally under The ISC License
        // https://github.com/npm/node-semver/blob/3a8a4309ae986c1967b3073ba88c9e69433d44cb/test/fixtures/comparisons.js

        test("0.0.0", "0.0.0-foo");
        test("0.0.1", "0.0.0");
        test("1.0.0", "0.9.9");
        test("0.10.0", "0.9.0");
        //test("0.99.0", "0.10.0", {});
        //test("2.0.0", "1.2.3", { loose: false });
        //test("v0.0.0", "0.0.0-foo", true);
        //test("v0.0.1", "0.0.0", { loose: true });
        //test("v1.0.0", "0.9.9", true);
        //test("v0.10.0", "0.9.0", true);
        //test("v0.99.0", "0.10.0", true);
        //test("v2.0.0", "1.2.3", true);
        //test("0.0.0", "v0.0.0-foo", true);
        //test("0.0.1", "v0.0.0", true);
        //test("1.0.0", "v0.9.9", true);
        //test("0.10.0", "v0.9.0", true);
        //test("0.99.0", "v0.10.0", true);
        //test("2.0.0", "v1.2.3", true);
        test("1.2.3", "1.2.3-asdf");
        test("1.2.3", "1.2.3-4");
        test("1.2.3", "1.2.3-4-foo");
        test("1.2.3-5-foo", "1.2.3-5");
        test("1.2.3-5", "1.2.3-4");
        test("1.2.3-5-foo", "1.2.3-5-Foo");
        test("3.0.0", "2.7.2+asdf");
        test("1.2.3-a.10", "1.2.3-a.5");
        test("1.2.3-a.b", "1.2.3-a.5");
        test("1.2.3-a.b", "1.2.3-a");
        test("1.2.3-a.b.c.10.d.5", "1.2.3-a.b.c.5.d.100");
        test("1.2.3-r2", "1.2.3-r100");
        test("1.2.3-r100", "1.2.3-R2");
    }
}
