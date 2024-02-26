use crate::build_target_info::{TargetFamily, TargetInformation};

mod build_target_info;

fn main() {
    tauri_build::build();

    let target_info = TargetInformation::from_triple(std::env::var("TARGET").unwrap().as_str());

    if target_info.family == TargetFamily::Linux {
        // start stop gc is not supported by dotnet.
        println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
    } else if target_info.family == TargetFamily::Windows {
        // "/merge:.modules=.rdata" "/merge:.unbox=.text"
        println!("cargo:rustc-link-arg=/merge:.modules=.rdata");
        println!("cargo:rustc-link-arg=/merge:.unbox=.text");
    }
}
