#![allow(dead_code)]

// Remember to update RuntimeFrameworkVersion in csproj file when updating this version
pub static FRAMEWORK_VERSION: &str = "8.0.1";

pub struct TargetInformation {
    pub dotnet_runtime_id: &'static str,
    pub output_file_name: &'static str,
    pub link_libraries: Vec<&'static str>,
    pub bootstrapper: &'static str,
    pub family: TargetFamily,
    pub patch_mach_o: bool,
    pub remove_libunwind: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFamily {
    Windows,
    Linux,
    MacOS,
}

impl TargetInformation {
    pub fn from_triple(triple: &str) -> Self {
        match triple {
            "x86_64-apple-darwin" => Self::macos("osx-x64"),
            "aarch64-apple-darwin" => Self::macos("osx-arm64"),

            "x86_64-pc-windows-msvc" => {
                let mut base = Self::windows("win-x64");
                base.link_libraries.push("static=Runtime.VxsortDisabled");
                base
            }
            "aarch64-pc-windows-msvc" => Self::windows("win-arm64"),

            "x86_64-unknown-linux-gnu" => Self::linux("linux-x64", false),
            "x86_64-unknown-linux-musl" => Self::linux("linux-musl-x64", true),
            "aarch64-unknown-linux-gnu" => Self::linux("linux-arm64", false),
            "aarch64-unknown-linux-musl" => Self::linux("linux-musl-arm64", true),

            _ => panic!("unsupported target triple: {}", triple),
        }
    }

    fn linux(rid: &'static str, remove_libunwind: bool) -> Self {
        Self {
            dotnet_runtime_id: rid,
            output_file_name: "vrc-get-litedb.a",
            link_libraries: vec!["static=System.Native", "static=stdc++compat"],
            bootstrapper: "libbootstrapperdll.o",
            patch_mach_o: false,
            family: TargetFamily::Linux,
            remove_libunwind,
        }
    }

    fn macos(rid: &'static str) -> Self {
        Self {
            dotnet_runtime_id: rid,
            output_file_name: "vrc-get-litedb.a",
            link_libraries: vec![
                "static=System.Native",
                "static=stdc++compat",
                "framework=Foundation",
            ],
            bootstrapper: "libbootstrapperdll.o",
            patch_mach_o: true,
            family: TargetFamily::MacOS,
            remove_libunwind: false,
        }
    }

    fn windows(rid: &'static str) -> Self {
        Self {
            dotnet_runtime_id: rid,
            output_file_name: "vrc-get-litedb.lib",
            link_libraries: vec![
                // windows sdk items
                "dylib=advapi32",
                "dylib=bcrypt",
                "dylib=crypt32",
                "dylib=iphlpapi",
                "dylib=kernel32",
                "dylib=mswsock",
                "dylib=ncrypt",
                "dylib=normaliz",
                "dylib=ntdll",
                "dylib=ole32",
                "dylib=oleaut32",
                "dylib=secur32",
                "dylib=user32",
                "dylib=version",
                "dylib=ws2_32",
            ],
            bootstrapper: "bootstrapperdll.obj",
            patch_mach_o: false,
            family: TargetFamily::Windows,
            remove_libunwind: false,
        }
    }
}
