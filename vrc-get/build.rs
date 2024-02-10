mod build_target_info;

use crate::build_target_info::*;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_FEATURE_EXPERIMENTAL_VCC").is_ok() {
        let target_info = TargetInformation::from_triple(std::env::var("TARGET").unwrap().as_str());

        println!("cargo:rustc-link-arg={home}/.nuget/packages/runtime.{target}.microsoft.dotnet.ilcompiler/{version}/sdk/{lib}",
                 home = home::home_dir().unwrap().to_str().unwrap(),
                 target = target_info.dotnet_runtime_id,
                 version = FRAMEWORK_VERSION,
                 lib = target_info.bootstrapper
        );

        if target_info.family == TargetFamily::Linux {
            // start stop gc is not supported by dotnet.
            println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
        }
    }
}
