use crate::data::manifest::{Manifest, MANIFEST_FILE_NAME};
use crate::dataflow::installed_manifest_packages::InstalledManifestPackages;
use crate::dataflow::{PackageKey, WapmPackageKey};
use std::borrow::Cow;
use std::collections::hash_set::HashSet;
use std::fs;
use std::path::Path;
use toml::Value;

#[derive(Clone, Debug, Fail)]
pub enum Error {
    #[fail(display = "Could not parse manifest because {}.", _0)]
    ManifestTomlParseError(String),
    #[fail(display = "Could not parse manifest because {}.", _0)]
    IoError(String),
    #[fail(display = "Dependency version must be a string. Package name: {}.", _0)]
    DependencyVersionMustBeString(String),
    #[fail(display = "Could not save manifest file because {}.", _0)]
    SaveError(String),
}

/// A ternary for a manifest: Some, None, Error.
#[derive(Debug)]
pub enum ManifestResult {
    Manifest(Manifest),
    NoManifest,
    ManifestError(Error),
}

impl ManifestResult {
    pub fn find_in_directory<P: AsRef<Path>>(directory: P) -> Self {
        let directory = directory.as_ref();
        if !directory.is_dir() {
            ManifestResult::ManifestError(Error::IoError(
                "Manifest must be a file named `wapm.toml`.".to_string(),
            ));
        }
        let manifest_path_buf = directory.join(MANIFEST_FILE_NAME);
        if !manifest_path_buf.is_file() {
            ManifestResult::ManifestError(Error::IoError(
                "Manifest must be a file named `wapm.toml`.".to_string(),
            ));
        }
        let source = match fs::read_to_string(&manifest_path_buf) {
            Ok(s) => s,
            Err(_) => return ManifestResult::NoManifest,
        };
        match toml::from_str::<Manifest>(&source) {
            Ok(m) => ManifestResult::Manifest(m),
            Err(e) => ManifestResult::ManifestError(Error::ManifestTomlParseError(e.to_string())),
        }
    }

    pub fn update_manifest(
        &self,
        installed_packages: &InstalledManifestPackages,
    ) -> Result<(), Error> {
        match self {
            ManifestResult::Manifest(ref m) if installed_packages.packages.len() > 0 => {
                let mut manifest = m.clone();
                for (key, _, _) in installed_packages.packages.iter() {
                    manifest.add_dependency(key.name.as_ref(), key.version.as_ref());
                }
                manifest.save().map_err(|e| Error::SaveError(e.to_string()))
            }
            _ => Ok(()),
        }
    }
}

/// A convenient structure containing all modules and commands for a package stored in manifest.
#[derive(Clone, Debug)]
pub struct ManifestPackages<'a> {
    pub package_keys: Option<HashSet<PackageKey<'a>>>,
}

impl<'a> ManifestPackages<'a> {
    pub fn new_from_result(result: &'a ManifestResult) -> Result<Self, Error> {
        match result {
            ManifestResult::Manifest(ref manifest) => Self::new_from_manifest(manifest),
            ManifestResult::NoManifest => Ok(Self { package_keys: None }),
            ManifestResult::ManifestError(e) => Err(e.clone()),
        }
    }

    fn new_from_manifest(manifest: &'a Manifest) -> Result<Self, Error> {
        match manifest {
            Manifest {
                dependencies: Some(ref dependencies),
                ..
            } => dependencies
                .iter()
                .map(|(name, value)| match value {
                    Value::String(ref version) => Ok(PackageKey::WapmPackage(WapmPackageKey {
                        name: Cow::Borrowed(name),
                        version: Cow::Borrowed(version),
                    })),
                    _ => Err(Error::DependencyVersionMustBeString(name.to_string())),
                })
                .collect::<Result<HashSet<PackageKey>, Error>>()
                .map(|package_keys| Self {
                    package_keys: Some(package_keys),
                }),
            _ => Ok(Self { package_keys: None }),
        }
    }

    pub fn add_additional_packages(&mut self, added_packages: Vec<(&'a str, &'a str)>) {
        if let Some(ref mut package_keys) = self.package_keys {
            for (name, version) in added_packages {
                let key = PackageKey::new_registry_package(name, version);
                package_keys.insert(key);
            }
        } else {
            self.package_keys = Some(
                added_packages
                    .into_iter()
                    .map(|(n, v)| PackageKey::new_registry_package(n, v))
                    .collect::<HashSet<_>>(),
            );
        }
    }

    pub fn keys(&self) -> HashSet<PackageKey<'a>> {
        self.package_keys
            .as_ref()
            .map(|m| m.iter().cloned().collect())
            .unwrap_or_default()
    }
}
