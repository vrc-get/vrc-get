use std::ffi::{OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb.csproj");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb/src");

    // currently this code is only tested on macOS.

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());

    let dotnet_built = build_dotnet(&out_dir, &manifest_dir);

    let dotnet_out_folder = out_dir.join("dotnet").join("bin/");
    let dotnet_sdk_folder = dotnet_out_folder.join("sdk");
    let dotnet_framework_folder = dotnet_out_folder.join("framework");

    println!("cargo:rustc-link-search={path}", path = dotnet_sdk_folder.display());
    println!(
        "cargo:rustc-link-search={path}",
        path = dotnet_framework_folder.display()
    );

    let bootstrapper = dotnet_sdk_folder.join("libbootstrapperdll.o");
    println!("cargo:rustc-link-arg={path}", path = bootstrapper.display());

    // link prebuilt dotnet
    if target_vendor == "apple" {
        // for apple platform, we need to fix object file a little
        // see https://github.com/dotnet/runtime/issues/96663

        let dst_object_file = out_dir.join("vrc-get-litedb-native-patched.o");
        patch_mach_o_from_archive(&dotnet_built, &dst_object_file);
        println!(
            "cargo:rustc-link-arg={path}",
            path = dst_object_file.display()
        );
    } else {
        println!(
            "cargo:rustc-link-lib=static:+verbatim={}",
            dotnet_built.display()
        );
    }

    let libs: &[&str] = &[
        // .NET runtime
        "System.Native",
        "Runtime.ServerGC",
        "stdc++compat",
        "System.Globalization.Native",
        "eventpipe-disabled",
    ];

    for x in libs {
        println!("cargo:rustc-link-lib=static={}", x);
    }

    if target_vendor == "apple" {
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}

fn build_dotnet(out_dir: &Path, manifest_dir: &Path) -> PathBuf {
    /*
    dotnet publish vrc-get-litedb.csproj -P:OutputPath="~/IdeaProjects/vrc-get/vrc-get-litedb/dotnet/target/bin/" -r osx-arm64
     */

    let mut command = Command::new("dotnet");
    command.arg("publish");
    command.arg(manifest_dir.join("dotnet/vrc-get-litedb.csproj"));

    // set output paths
    let output_dir = out_dir.join("dotnet").join("bin/");
    command.arg("--output").arg(&output_dir);
    let mut building = OsString::from("-P:IntermediateOutputPath=");
    building.push(out_dir.join("dotnet").join("obj/"));
    command.arg(building);

    // os
    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        #[cfg(target_os = "macos")]
        "macos" => {
            command.arg("--os").arg("osx");
        }
        #[cfg(target_os = "windows")]
        "windows" => {
            command.arg("--os").arg("win");
        }
        #[cfg(target_os = "linux")]
        "linux" => {
            command.arg("--os").arg("linux");
        }
        os => panic!("unsupported target os: {os}. cross-os building is not supported by .NET nativeaot."),
    }

    // arch
    match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => {
            command.arg("--arch").arg("x64");
        }
        "aarch64" => {
            command.arg("--arch").arg("arm64");
        }
        arch => panic!("unsupported target arch: {arch}"),
    }

    let status = command.status().unwrap();
    if !status.success() {
        panic!("failed to build dotnet library");
    }

    match std::env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => {
            output_dir.join("vrc-get-litedb.a")
        }
        "windows" => {
            output_dir.join("vrc-get-litedb.lib")
        }
        "linux" => {
            output_dir.join("vrc-get-litedb.a")
        }
        os => panic!("unsupported target os: {os}"),
    }
}

fn patch_mach_o_from_archive(archive: &Path, dst_object_file: &Path) {
    std::fs::remove_file(&dst_object_file).ok();

    let file = std::fs::File::open(archive).expect("failed to open built library");
    let mut archive = ar::Archive::new(file);
    while let Some(entry) = archive.next_entry() {
        let mut entry = entry.expect("reading library");
        if entry.header().identifier().ends_with(b".o") {
            use object::endian::*;
            use object::from_bytes;
            use object::macho::*;

            // it's object file
            let mut dst_file = std::fs::File::options()
                .create_new(true)
                .read(true)
                .write(true)
                .open(&dst_object_file)
                .expect("creating patched object file");
            std::io::copy(&mut entry, &mut dst_file).expect("creating patched object file");
            dst_file.flush().expect("creating patched object file");

            let mut mapped = unsafe { memmap2::MmapMut::map_mut(&dst_file) }
                .expect("mmap: patching object file");
            let as_slice = &mut mapped[..];

            let (magic, _) = from_bytes::<U32<BigEndian>>(as_slice).unwrap();
            if magic.get(BigEndian) == MH_MAGIC_64 {
                patch_mach_o_64(as_slice, Endianness::Big);
            } else if magic.get(BigEndian) == MH_CIGAM_64 {
                patch_mach_o_64(as_slice, Endianness::Little);
            } else {
                panic!("invalid mach-o: unknown magic");
            }

            mapped.flush().expect("flush:patching object file");
            drop(mapped);
            drop(dst_file);
        }
    }
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
                if &section_header.sectname == b"hydrated\0\0\0\0\0\0\0\0"
                    && &section_header.segname == b"__DATA\0\0\0\0\0\0\0\0\0\0"
                {
                    // hydrated section in the data segment
                    let flags = section_header.flags.get(endian);
                    let flags = flags | S_ATTR_NO_DEAD_STRIP;
                    section_header.flags.set(endian, flags);
                } else if &section_header.sectname == b"__modules\0\0\0\0\0\0\0"
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
