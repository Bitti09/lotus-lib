/*!

This module provides functionality to read and parse the packages from the `Cache.Windows`
directory. The `Cache.Windows` directory contains the packages that store the data for Warframe and Soulframe.

*/

mod package;
mod package_collection;
mod package_type;

pub use package::Package;
pub use package_collection::PackageCollection;
pub use package_type::PackageType;
