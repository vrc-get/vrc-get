fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[allow(clippy::collapsible_if)]
    if std::env::var("CARGO_FEATURE_EXPERIMENTAL_VCC").is_ok() {
        if std::env::var("TARGET").unwrap().contains("linux") {
            // start stop gc is not supported by dotnet.
            println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
        }
    }
}
