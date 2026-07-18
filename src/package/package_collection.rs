use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use crate::cache_pair::CachePair;
use crate::package::package::Package;

/// Supported localization suffixes.
pub const LOCALIZATION_SUFFIXES: &[&str] = &[
    "_en", "_de", "_fr", "_it", "_es", "_ja", "_ko", "_pl", "_pt", "_ru", "_tr", "_uk", "_zh",
    "_xx",
];

/// A collection of packages.
pub struct PackageCollection<T: CachePair> {
    directory: PathBuf,
    is_post_ensmallening: bool,
    packages: HashMap<String, Package<T>>,
}

impl<T: CachePair> PackageCollection<T> {
    /// Creates a new package collection from the specified directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory does not exist or if the directory cannot be read.
    pub fn new<P>(directory: P, is_post_ensmallening: bool) -> Result<Self, io::Error>
    where
        P: Into<PathBuf>,
    {
        let directory = directory.into();

        let mut packages = HashMap::new();
        for entry in std::fs::read_dir(&directory).unwrap() {
            let entry = entry?;
            let file_name = entry.file_name().into_string().unwrap();

            // Check if the file has enough characters to be a package 7 characters counts for the
            // shortest possible package name of 1 character : H.1.toc
            if file_name.len() < 7 {
                continue;
            }

            // Check if the file is a valid header .toc file
            if !file_name.starts_with("H.") || !file_name.ends_with(".toc") {
                continue;
            }

            let package_name = file_name[2..file_name.len() - 4].to_string();

            let package = Package::<T>::new(&directory, package_name.clone(), is_post_ensmallening);
            packages.insert(package_name, package);
        }

        Ok(Self {
            directory,
            is_post_ensmallening,
            packages,
        })
    }

    /// Returns whether the package is post-ensmallening.
    ///
    /// This is used to determine how to decompress the data from before "The Great Ensmallening"
    /// update of Warframe. Also applies to Soulframe.
    pub fn is_post_ensmallening(&self) -> bool {
        self.is_post_ensmallening
    }

    /// Returns a reference to the package with the specified name if found.
    pub fn borrow(&self, package_name: &str) -> Option<&Package<T>> {
        self.packages.get(package_name)
    }

    /// Returns a mutable reference to the package with the specified name if found.
    pub fn borrow_mut(&mut self, package_name: &str) -> Option<&mut Package<T>> {
        self.packages.get_mut(package_name)
    }

    /// Returns the package with the specified name if found.
    pub fn take(&mut self, package_name: &str) -> Option<Package<T>> {
        self.packages.remove(package_name)
    }

    /// Returns the directory of the package collection.
    pub fn directory(&self) -> &PathBuf {
        &self.directory
    }

    /// Returns the packages within the package collection.
    pub fn packages(&self) -> &HashMap<String, Package<T>> {
        &self.packages
    }

    /// Check if a package name has a localization suffix.
    pub fn is_localized_package(package_name: &str) -> bool {
        LOCALIZATION_SUFFIXES
            .iter()
            .any(|suffix| package_name.ends_with(suffix))
    }

    /// Get the base package name without localization suffix.
    pub fn get_base_package_name(package_name: &str) -> &str {
        for suffix in LOCALIZATION_SUFFIXES {
            if let Some(stripped) = package_name.strip_suffix(suffix) {
                return stripped;
            }
        }
        package_name
    }

    /// Get all localization variants for a base package name.
    pub fn get_localization_variants<'a>(
        &'a self,
        base_name: &'a str,
    ) -> Vec<(&'a str, &'a Package<T>)> {
        let mut variants = Vec::new();

        if let Some(pkg) = self.packages.get(base_name) {
            variants.push((base_name, pkg));
        }

        for suffix in LOCALIZATION_SUFFIXES {
            let localized_key = self
                .packages
                .keys()
                .find(|k| k.as_str() == format!("{}{}", base_name, suffix).as_str());
            if let Some(key) = localized_key {
                if let Some(pkg) = self.packages.get(key.as_str()) {
                    variants.push((key.as_str(), pkg));
                }
            }
        }

        variants
    }
}
