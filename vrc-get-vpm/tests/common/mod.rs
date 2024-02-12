// since this moduke is shared between multiple tests, we need to allow dead code and unused imports
#![allow(dead_code)]
#![allow(unused_imports)]

mod package_collection;
mod virtual_file_system;

pub(crate) use package_collection::PackageCollection;
pub(crate) use virtual_file_system::VirtualFileSystem;
