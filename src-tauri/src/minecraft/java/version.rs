use std::cmp::Ordering;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct JavaVersion {
    numbers: Vec<u32>,
    build: Option<u32>,
}

impl JavaVersion {
    pub(super) fn new(mut numbers: Vec<u32>, build: Option<u32>) -> Option<Self> {
        if !numbers.first().is_some_and(|&major| major > 0) {
            return None;
        }
        while numbers.last() == Some(&0) {
            numbers.pop();
        }
        Some(Self { numbers, build })
    }

    pub fn parse(text: &str) -> Option<Self> {
        if let Some(update) = text.strip_prefix("1.8.0_") {
            let (update, build) = match update.split_once("-b") {
                Some((update, build)) => (update, Some(build.parse().ok()?)),
                None => (update, None),
            };
            return Self::new(vec![8, 0, update.parse().ok()?], build);
        }

        let (version, build) = match text.split_once('+') {
            Some((version, suffix)) => {
                let build = suffix.split('-').next()?.parse().ok()?;
                (version, Some(build))
            }
            None => (text.strip_suffix("-LTS").unwrap_or(text), None),
        };
        let numbers = version
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        Self::new(numbers, build)
    }

    pub fn major(&self) -> u32 {
        self.numbers[0]
    }

    pub fn matches(&self, installed: &Self) -> bool {
        self.numbers == installed.numbers && (self.build.is_none() || self.build == installed.build)
    }

    pub fn is_newer_than(&self, installed: &Self) -> bool {
        if self.major() != installed.major() {
            return false;
        }
        match self.numbers.cmp(&installed.numbers) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                matches!((self.build, installed.build), (Some(new), Some(old)) if new > old)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> JavaVersion {
        JavaVersion::parse(text).unwrap()
    }

    #[test]
    fn orders_stable_builds_within_a_major() {
        assert_eq!(version("1.8.0_512-b09"), version("8.0.512+9"));
        assert!(version("1.8.0_512-b10").is_newer_than(&version("1.8.0_512-b09")));
        assert!(version("21.0.13.1+2-LTS").is_newer_than(&version("21.0.13+11")));
        assert!(!version("21+17").is_newer_than(&version("17+35")));
        assert!(version("21.0.0+35").matches(&version("21+35-LTS")));
        assert!(!version("21.0.13+9").matches(&version("21.0.13+8")));
    }

    #[test]
    fn graal_catalog_has_patch_but_not_vm_build() {
        let catalog = version("21.0.12.1.1");
        let installed = version("21.0.12.1.1+1-LTS-jvmci-23.1-b99");
        assert!(catalog.matches(&installed));
        assert!(!catalog.is_newer_than(&installed));
        assert!(catalog.is_newer_than(&version("21.0.12+8-LTS")));
        assert!(!catalog.is_newer_than(&version("21.0.13.1+1-LTS")));
    }

    #[test]
    fn rejects_prereleases_and_malformed_versions() {
        for text in [
            "21-ea+35",
            "25-rc",
            "1.8.0_512-ea",
            "21.1beta",
            "21.0+bad",
            "0",
            "",
        ] {
            assert!(JavaVersion::parse(text).is_none(), "{text}");
        }
    }
}
