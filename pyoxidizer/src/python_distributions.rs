// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Defines known Python distributions.

use {
    crate::py_packaging::distribution::{DistributionFlavor, PythonDistributionRecord},
    itertools::Itertools,
};

pub use crate::default_python_distributions::PYTHON_DISTRIBUTIONS;

/// Default Python X.Y version to use.
pub const DEFAULT_PYTHON_VERSION: &str = "3.10";

/// A collection of available Python distributions.
pub struct PythonDistributionCollection {
    pub(crate) dists: Vec<PythonDistributionRecord>,
}

impl PythonDistributionCollection {
    /// Find a Python distribution given requirements.
    ///
    /// `target_triple` is the Rust machine triple the distribution is built for.
    /// `flavor` is the type of Python distribution.
    /// `python_major_minor_version` is an optional `X.Y` version string being
    /// requested. If `None`, `DEFAULT_PYTHON_VERSION` is used. If no
    /// distribution of that version exists for the target (some targets are
    /// only published for newer Python versions), the oldest available
    /// version is used instead.
    pub fn find_distribution(
        &self,
        target_triple: &str,
        flavor: &DistributionFlavor,
        python_major_minor_version: Option<&str>,
    ) -> Option<PythonDistributionRecord> {
        let candidates = self
            .dists
            .iter()
            .filter(|dist| dist.target_triple == target_triple)
            .filter(|dist| match flavor {
                DistributionFlavor::Standalone => true,
                DistributionFlavor::StandaloneStatic => !dist.supports_prebuilt_extension_modules,
                DistributionFlavor::StandaloneDynamic => dist.supports_prebuilt_extension_modules,
            })
            .collect::<Vec<_>>();

        let find = |version: &str| {
            candidates
                .iter()
                .find(|dist| dist.python_major_minor_version == version)
                .map(|dist| (*dist).clone())
        };

        match python_major_minor_version {
            Some(version) => find(version),
            None => find(DEFAULT_PYTHON_VERSION).or_else(|| {
                candidates
                    .iter()
                    .min_by_key(|dist| parse_major_minor(&dist.python_major_minor_version))
                    .map(|dist| (*dist).clone())
            }),
        }
    }

    /// Obtain records for all registered distributions.
    #[allow(unused)]
    pub fn iter(&self) -> impl Iterator<Item = &PythonDistributionRecord> {
        self.dists.iter()
    }

    /// All target triples of distributions in this collection.
    #[allow(unused)]
    pub fn all_target_triples(&self) -> impl Iterator<Item = &str> {
        self.dists
            .iter()
            .map(|dist| dist.target_triple.as_str())
            .sorted()
            .dedup()
    }
}

/// Parse an `X.Y` version string into a sortable tuple.
fn parse_major_minor(version: &str) -> (u32, u32) {
    let mut parts = version.split('.').map(|p| p.parse::<u32>().unwrap_or(0));

    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_distribution_default_version() {
        let dist = PYTHON_DISTRIBUTIONS
            .find_distribution(
                "x86_64-unknown-linux-gnu",
                &DistributionFlavor::Standalone,
                None,
            )
            .unwrap();
        assert_eq!(dist.python_major_minor_version, DEFAULT_PYTHON_VERSION);

        // aarch64 Windows distributions start at 3.11, so the oldest
        // available version is used when no version is requested.
        let dist = PYTHON_DISTRIBUTIONS
            .find_distribution(
                "aarch64-pc-windows-msvc",
                &DistributionFlavor::Standalone,
                None,
            )
            .unwrap();
        assert_eq!(dist.python_major_minor_version, "3.11");

        // An explicit version is never substituted.
        assert!(PYTHON_DISTRIBUTIONS
            .find_distribution(
                "aarch64-pc-windows-msvc",
                &DistributionFlavor::Standalone,
                Some("3.10"),
            )
            .is_none());
        assert_eq!(
            PYTHON_DISTRIBUTIONS
                .find_distribution(
                    "aarch64-pc-windows-msvc",
                    &DistributionFlavor::Standalone,
                    Some("3.14"),
                )
                .unwrap()
                .python_major_minor_version,
            "3.14"
        );

        assert!(PYTHON_DISTRIBUTIONS
            .find_distribution("unknown-triple", &DistributionFlavor::Standalone, None)
            .is_none());
    }

    #[test]
    fn test_all_target_triples() {
        assert_eq!(
            PYTHON_DISTRIBUTIONS
                .all_target_triples()
                .collect::<Vec<_>>(),
            vec![
                "aarch64-apple-darwin",
                "aarch64-pc-windows-msvc",
                "aarch64-unknown-linux-gnu",
                "aarch64-unknown-linux-musl",
                "i686-pc-windows-msvc",
                "x86_64-apple-darwin",
                "x86_64-pc-windows-msvc",
                "x86_64-unknown-linux-gnu",
                "x86_64-unknown-linux-musl",
                "x86_64_v2-unknown-linux-gnu",
                "x86_64_v2-unknown-linux-musl",
                "x86_64_v3-unknown-linux-gnu",
                "x86_64_v3-unknown-linux-musl",
                "x86_64_v4-unknown-linux-gnu",
            ]
        );
    }
}
