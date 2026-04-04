use anyhow::*;
use itertools::Itertools as _;
use object::{Endian, Endianness, FileKind, Object};
use std::fs;
use std::result::Result::Ok;

/// Checks if the binary or binaries in the specified directory have external dependencies.
///
/// This tool checks the binary or binaries in the specified directory have dynamically linked
/// library dependencies that are not part of Operating System.
///
/// Depending on the binary format this command behaves differently:
/// - For ELF images, that are for Linux, this tool actually checks if statically linked
///   by checking if import table is empty.
/// - For Mach-O images, that are for macOS, this tool ensures all dynamically linked dependencies
///   are known dylib's or frameworks that are installed initially.
///   This includes libSystem.B.dylib or Security.framework.
/// - For PE64 images, that are for Windows, this tool ensures all dynamically linked dependencies
///   are known system dlls that are installed initially for long time.
///   This includes kernel32.dll but not api-ms-win-crt-private-l1-1-0.dll.
#[derive(clap::Parser)]
#[clap(verbatim_doc_comment)]
pub(super) struct Command {
    files: Vec<std::path::PathBuf>,
}

impl crate::Command for Command {
    fn run(self) -> Result<i32> {
        let mut all_static = true;
        let mut error = false;
        for p in self.files {
            let files = if p.is_file() {
                vec![p]
            } else if p.is_dir() {
                fs::read_dir(p)?
                    .map_ok(|x| x.path())
                    .filter_ok(|x| x.is_file())
                    .filter_ok(|x| !x.file_name().unwrap().as_encoded_bytes().starts_with(b"."))
                    .filter_ok(|x| x.extension() != Some("d".as_ref()))
                    .filter_ok(|x| x.extension() != Some("rlib".as_ref()))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                error = true;
                eprintln!("No such file or directory {}", p.display());
                continue;
            };

            for file in files {
                all_static &= match check_static_link(&file) {
                    Ok(false) => false,
                    Ok(true) => {
                        println!("No non-os dependencies found: {}", file.display());
                        true
                    }
                    Err(e) => {
                        eprintln!("Failed to check static link {}: {:#}", file.display(), e);
                        error = true;
                        true
                    }
                }
            }
        }

        if error {
            Ok(2)
        } else if all_static {
            Ok(0)
        } else {
            Ok(1)
        }
    }
}

pub fn check_static_link(path: &std::path::Path) -> Result<bool> {
    let binary = fs::read(path).context("Reading binary")?;

    Ok(
        match FileKind::parse(binary.as_slice()).context("detecting type")? {
            FileKind::MachO64 => process_mach_64::<Endianness>(&binary)?,
            FileKind::Pe64 => process_pe_64(&binary)?,
            FileKind::Elf64 => process_elf_64::<Endianness>(&binary)?,
            unknown => bail!("unsupported file type: {unknown:?}"),
        },
    )
}

fn process_mach_64<E: Endian>(binary: &[u8]) -> Result<bool> {
    use object::macho::*;
    use object::read::macho::*;

    let mut success = true;

    let parsed = MachHeader64::<E>::parse(binary, 0).context("failed to parse binary")?;
    let endian = parsed.endian()?;

    let mut commands = parsed.load_commands(endian, binary, 0)?;
    while let Some(command) = commands.next()? {
        if let Some(dylib) = command.dylib()? {
            let dylib = command.string(endian, dylib.dylib.name)?;
            match dylib {
                | b"/System/Library/Frameworks/Security.framework/Versions/A/Security"
                | b"/System/Library/Frameworks/SystemConfiguration.framework/Versions/A/SystemConfiguration"
                | b"/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation"
                | b"/System/Library/Frameworks/Foundation.framework/Versions/C/Foundation"
                | b"/usr/lib/libobjc.A.dylib"
                | b"/usr/lib/libiconv.2.dylib"
                | b"/usr/lib/libSystem.B.dylib"
                => {
                    // known system library
                    println!("system dylib: {}", std::str::from_utf8(dylib)?);
                }
                unknown => {
                    println!("ERROR: unknown dylib: {:?}", std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8"));
                    success = false;
                },
            }
        } else if command.cmd() == LC_LOAD_DYLINKER {
            let data: &DylinkerCommand<E> = command.data().expect("parse LC_LOAD_DYLINKER");
            if command.string(endian, data.name)? != b"/usr/lib/dyld" {
                println!("ERROR: dylinker is not /usr/lib/dyld");
                success = false;
            } else {
                println!("dylinker: /usr/lib/dyld");
            }
        }
    }
    Ok(success)
}

fn process_pe_64(binary: &[u8]) -> Result<bool> {
    use object::LittleEndian as LE;
    use object::read::pe::*;

    let mut success = true;
    let parsed = PeFile64::parse(binary).context("failed to parse binary")?;

    let table = parsed.import_table()?.unwrap();
    let mut iter = table.descriptors()?;
    while let Some(x) = iter.next()? {
        let dll = table.name(x.name.get(LE))?;
        match dll.to_ascii_lowercase().as_slice() {
            // all sinces are came from AI
            b"advapi32.dll" // since Windows 95
            | b"kernel32.dll" // since Windows 95 / NT 4.0
            | b"bcrypt.dll" // since Windows Vista / Server 2008
            | b"ntdll.dll" // since Windows NT 3.1
            | b"shell32.dll" // since Windows 95/NT 4.0
            | b"ole32.dll" // since Windows 95 era
            | b"ws2_32.dll" // since Windows 95 OSR2
            | b"crypt32.dll" // since Windows NT 4.0/XP era
            | b"bcryptprimitives.dll" // since Windows 7/Server 2008 R2
            | b"combase.dll" // since Windows 8 / Server 2012
            | b"api-ms-win-core-synch-l1-2-0.dll" // since Windows 8 / Server 2012
            => {
                println!(
                    "system dll: {}",
                    std::str::from_utf8(dll).unwrap_or("unable to parse with utf8")
                );
                // known system library
            }
            unknown => {
                println!(
                    "ERROR: unknown dll: {:?}",
                    std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8")
                );
                success = false;
            }
        }
    }

    Ok(success)
}

fn process_elf_64<E: Endian>(binary: &[u8]) -> Result<bool> {
    use object::elf::*;
    use object::read::elf::*;

    let mut success = true;

    let parsed = ElfFile64::<E>::parse(binary).context("failed to parse binary")?;

    for x in parsed.imports()? {
        println!(
            "dynamic importing symbol: {}",
            std::str::from_utf8(x.name()).unwrap_or("<unknown>")
        );
        success = false;
    }

    for segment in parsed.elf_program_headers() {
        if segment.p_type.get(parsed.endian()) == PT_INTERP {
            let data = segment.data(parsed.endian(), parsed.data()).unwrap();
            println!(
                "interpreter: {:?}",
                std::str::from_utf8(data).unwrap_or("<unknown>")
            );
            success = false;
        }
    }

    Ok(success)
}
