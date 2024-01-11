use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb.csproj");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb/src");

    // currently this code is only tested on macOS.

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let target_info = TargetInformation::from_triple(std::env::var("TARGET").unwrap().as_str());
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());

    let dotnet_out_folder = build_dotnet(&out_dir, &manifest_dir, &target_info);

    let dotnet_built = dotnet_out_folder.join(target_info.output_file_name);
    let dotnet_sdk_folder = dotnet_out_folder.join("sdk");
    let dotnet_framework_folder = dotnet_out_folder.join("framework");

    let patched_lib_folder = out_dir.join("patched-lib");
    std::fs::create_dir_all(&patched_lib_folder).expect("creating patched folder");

    println!(
        "cargo:rustc-link-search=native={path}",
        path = patched_lib_folder.display()
    );
    println!(
        "cargo:rustc-link-search=native={path}",
        path = dotnet_sdk_folder.display()
    );
    println!(
        "cargo:rustc-link-search=native={path}",
        path = dotnet_framework_folder.display()
    );
    println!(
        "cargo:rustc-link-search=native={path}",
        path = dotnet_built.parent().unwrap().display()
    );

    let bootstrapper = dotnet_sdk_folder.join(target_info.bootstrapper);
    println!("cargo:rustc-link-arg={path}", path = bootstrapper.display());

    // link prebuilt dotnet
    if target_info.family == TargetFamily::MacOS {
        // for apple platform, we need to fix object file a little
        // see https://github.com/dotnet/runtime/issues/96663

        let patched = patched_lib_folder.join("vrc-get-litedb-patched.a");
        patch_mach_o_from_archive(&dotnet_built, &patched);
        println!("cargo:rustc-link-lib=static:+verbatim=vrc-get-litedb-patched.a");
    } else {
        println!(
            "cargo:rustc-link-lib=static:+verbatim={}",
            dotnet_built.file_name().unwrap().to_string_lossy()
        );
    }

    if target_info.remove_libunwind {
        // for linux musl, duplicated linking libunwind causes linkage error so
        // strip from Runtime.WorkstationGC.a
        let lib_name = "libRuntime.WorkstationGC.a";
        let before = dotnet_sdk_folder.join(lib_name);
        let patched = patched_lib_folder.join(lib_name);
        remove_libunwind(&before, &patched);
    }

    if target_info.family == TargetFamily::Linux {
        // start stop gc is not supported by dotnet. 
        println!("cargo:rustc-link-arg=-Wl,-z,nostart-stop-gc");
    }

    let common_libs: &[&str] = &[
        //"static=Runtime.ServerGC",
        "static=Runtime.WorkstationGC",
        "static=eventpipe-disabled",
    ];

    for lib in common_libs {
        println!("cargo:rustc-link-lib={lib}");
    }

    for lib in target_info.link_libraries {
        println!("cargo:rustc-link-lib={lib}");
    }
}

struct TargetInformation {
    dotnet_runtime_id: &'static str,
    output_file_name: &'static str,
    link_libraries: &'static [&'static str],
    bootstrapper: &'static str,
    family: TargetFamily,
    patch_mach_o: bool,
    remove_libunwind: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetFamily {
    Windows,
    Linux,
    MacOS,
}

impl TargetInformation {
    fn from_triple(triple: &str) -> Self {
        match triple {
            "x86_64-apple-darwin" => Self::macos("osx-x64"),
            "aarch64-apple-darwin" => Self::macos("osx-arm64"),

            "x86_64-pc-windows-msvc" => Self::windows("win-x64"),
            "aaarch64-pc-windows-msvc" => Self::windows("win-arm64"),

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
            link_libraries: &[
                "static=System.Native",
                "static=System.Globalization.Native",
                "static=stdc++compat",
            ],
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
            link_libraries: &[
                "static=System.Native",
                "static=System.Globalization.Native",
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
            link_libraries: &[
                "static=System.Globalization.Native.Aot",
                "static=Runtime.VxsortDisabled",

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

fn build_dotnet(out_dir: &Path, manifest_dir: &Path, target: &TargetInformation) -> PathBuf {
    let mut command = Command::new("dotnet");
    command.arg("publish");
    command.arg(manifest_dir.join("dotnet/vrc-get-litedb.csproj"));

    // set output paths
    let output_dir = out_dir.join("dotnet").join("bin/");
    command.arg("--output").arg(&output_dir);
    let mut building = OsString::from("-p:IntermediateOutputPath=");
    building.push(out_dir.join("dotnet").join("obj/"));
    command.arg(building);

    command.arg("--runtime").arg(target.dotnet_runtime_id);

    if target.patch_mach_o {
        // according to filipnavara, setting S_ATTR_NO_DEAD_STRIP for hydrated section is invalid
        // so use IlcDehydrate=false instead
        command.arg("-p:IlcDehydrate=false");
    }

    let status = command.status().unwrap();
    if !status.success() {
        panic!("failed to build dotnet library");
    }

    output_dir
}

fn patch_mach_o_from_archive(archive: &Path, patched: &Path) {

    let file = std::fs::File::open(archive).expect("failed to open built library");
    let mut archive = ar::Archive::new(std::io::BufReader::new(file));

    let patched = std::fs::File::create(patched).expect("failed to create patched library");
    let mut builder = ar::Builder::new(std::io::BufWriter::new(patched));

    while let Some(entry) = archive.next_entry() {
        let mut entry = entry.expect("reading library");
        if entry.header().identifier().ends_with(b".o") {
            let mut buffer = vec![0u8; 0];

            std::io::copy(&mut entry, &mut buffer).expect("reading library");

            use object::endian::*;
            use object::from_bytes;
            use object::macho::*;

            let (magic, _) = from_bytes::<U32<BigEndian>>(&buffer).unwrap();
            if magic.get(BigEndian) == MH_MAGIC_64 {
                patch_mach_o_64(&mut buffer, Endianness::Big);
            } else if magic.get(BigEndian) == MH_CIGAM_64 {
                patch_mach_o_64(&mut buffer, Endianness::Little);
            } else {
                panic!("invalid mach-o: unknown magic");
            }

            builder.append(entry.header(), std::io::Cursor::new(buffer)).expect("copying file in archive");
        } else {
            builder.append(&entry.header().clone(), &mut entry).expect("copying file in archive");
        }
    }

    builder.into_inner().unwrap().flush().expect("writing patched library");
}

fn patch_mach_o_64<E: object::Endian>(as_slice: &mut [u8], endian: E) {
    use object::macho::*;
    use object::{from_bytes_mut, slice_from_bytes_mut};

    let (header, as_slice) = from_bytes_mut::<MachHeader64<E>>(as_slice).unwrap();
    let command_count = header.ncmds.get(endian);
    let mut as_slice = as_slice;
    for _ in 0..command_count {
        let (cmd, _) = from_bytes_mut::<LoadCommand<E>>(as_slice).unwrap();
        let cmd_size = cmd.cmdsize.get(endian) as usize;
        if cmd.cmd.get(endian) == LC_SEGMENT_64 {
            let data = &mut as_slice[..cmd_size];
            let (cmd, data) = from_bytes_mut::<SegmentCommand64<E>>(data).unwrap();
            let section_count = cmd.nsects.get(endian);
            let (section_headers, _) =
                slice_from_bytes_mut::<Section64<E>>(data, section_count as usize).unwrap();
            for section_header in section_headers {
                if &section_header.sectname == b"__modules\0\0\0\0\0\0\0"
                    && &section_header.segname == b"__DATA\0\0\0\0\0\0\0\0\0\0"
                {
                    // __modules section in the data segment
                    let flags = section_header.flags.get(endian);
                    let flags = flags | S_ATTR_NO_DEAD_STRIP;
                    section_header.flags.set(endian, flags);
                }
            }
        }
        as_slice = &mut as_slice[cmd_size..];
    }
}

fn remove_libunwind(archive: &Path, patched: &Path) {
    let file = std::fs::File::open(archive).expect("failed to open built library");
    let mut archive = ar::Archive::new(std::io::BufReader::new(file));

    let patched = std::fs::File::create(patched).expect("failed to create patched library");
    let mut builder = ar::Builder::new(std::io::BufWriter::new(patched));

    while let Some(entry) = archive.next_entry() {
        let mut entry = entry.expect("reading library");
        if entry.header().identifier().starts_with(b"libunwind") {
            // remove libunwind
        } else {
            builder.append(&entry.header().clone(), &mut entry).expect("copying file in archive");
        }
    }

    builder.into_inner().unwrap().flush().expect("writing patched library");
}
