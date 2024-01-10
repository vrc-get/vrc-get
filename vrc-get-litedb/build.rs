use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb.csproj");
    println!("cargo:rerun-if-changed=dotnet/vrc-get-litedb/src");

    // currently this code is only tested on macOS.

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let ilcompiler_path = PathBuf::from(std::env::var_os("DOTNET_ILCOMPILER_PACKAGE").expect(
        "please set path to runtime.<target>.microsoft.dotnet.ilcompiler package to DOTNET_ILCOMPILER_PACKAGE env var"
    ));
    let dontet_built = PathBuf::from(
        std::env::var_os("DOTNET_BUILT_LIBRARY")
            .expect("please set path to built static libarary to DOTNET_BUILT_LIBRARY env var"),
    );
    let target_vendor = std::env::var_os("CARGO_CFG_TARGET_VENDOR").unwrap();

    let sdk_path = ilcompiler_path.join("sdk");
    let framework_path = ilcompiler_path.join("framework");

    println!("cargo:rustc-link-search={path}", path = sdk_path.display());
    println!(
        "cargo:rustc-link-search={path}",
        path = framework_path.display()
    );

    let bootstrapper = sdk_path.join("libbootstrapperdll.o");
    println!("cargo:rustc-link-arg={path}", path = bootstrapper.display());

    // link prebuilt dotnet
    if target_vendor.as_os_str() == OsStr::new("apple") {
        // for apple platform, we need to fix object file a little
        // see https://github.com/dotnet/runtime/issues/96663

        let dst_object_file = out_dir.join("vrc-get-litedb-native-patched.o");
        patch_mach_o_from_archive(&dontet_built, &dst_object_file);
        println!(
            "cargo:rustc-link-arg={path}",
            path = dst_object_file.display()
        );
    } else {
        println!(
            "cargo:rustc-link-lib=static:+verbatim={}",
            dontet_built.display()
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

    if target_vendor.as_os_str() == OsStr::new("apple") {
        println!("cargo:rustc-link-lib=framework=Foundation");
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
