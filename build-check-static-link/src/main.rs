use std::fs;
use std::process::exit;
use object::{Endian, Endianness, FileKind, Object};

fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let binary = args.next().unwrap();
    let binary = std::path::Path::new(&binary);
    let binary = fs::read(binary).unwrap();

    let success = match FileKind::parse(binary.as_slice()).expect("detecting type") {
        FileKind::MachO64 => process_mach_64::<Endianness>(&binary),
        FileKind::Pe64 => process_pe_64(&binary),
        FileKind::Elf64 => process_elf_64::<Endianness>(&binary),
        unknown => panic!("unknown file type: {:?}", unknown),
    };

    if success {
        exit(0)
    } else {
        exit(1)
    }
}

fn process_mach_64<E : Endian>(binary: &[u8]) -> bool {
    use object::macho::*;
    use object::read::macho::*;

    let mut success = true;

    let parsed = MachHeader64::<E>::parse(binary, 0).expect("failed to parse binary");
    let endian = parsed.endian().unwrap();

    let mut commands = parsed.load_commands(endian, binary, 0).expect("parsing binary");
    while let Some(command) = commands.next().expect("reading binary") {
        match command.cmd() {
            | LC_SEGMENT_64
            | LC_DYLD_EXPORTS_TRIE
            | LC_DYLD_CHAINED_FIXUPS
            | LC_SYMTAB
            | LC_DYSYMTAB
            | LC_UUID
            | LC_BUILD_VERSION
            | LC_SOURCE_VERSION
            | LC_MAIN
            | LC_FUNCTION_STARTS
            | LC_DATA_IN_CODE
            | LC_CODE_SIGNATURE
            => {
                // ignore
            },
            LC_LOAD_DYLINKER => {
                let data: &DylinkerCommand<E> = command.data().expect("parse LC_LOAD_DYLINKER");
                if command.string(endian, data.name).unwrap() != b"/usr/lib/dyld" {
                    println!("ERROR: dylinker is not /usr/lib/dyld");
                    success = false;
                } else {
                    println!("dylinker: /usr/lib/dyld");
                }
            },
            LC_LOAD_DYLIB => {
                let data = command.dylib().expect("parse LC_LOAD_DYLIB").unwrap();
                let dylib = command.string(endian, data.dylib.name).unwrap();
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
                        println!("system dylib: {}", std::str::from_utf8(dylib).unwrap());
                    }
                    unknown => {
                        println!("ERROR: unknown dylib: {:?}", std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8"));
                        success = false;
                    },
                }
            },
            unknown => {
                println!("unknown linker command: {unknown:08x}");
                success = false;
            },
        }
    }
    success
}

fn process_pe_64(binary: &[u8]) -> bool {
    use object::read::pe::*;
    use object::LittleEndian as LE;

    let mut success = true;
    let parsed = PeFile64::parse(binary).expect("failed to parse binary");

    let table = parsed.import_table().unwrap().unwrap();
    let mut iter = table.descriptors().unwrap();
    while let Some(x) = iter.next().unwrap() {
        let dll = table.name(x.name.get(LE)).unwrap();
        match dll.to_ascii_lowercase().as_slice() {
            | b"advapi32.dll"
            | b"kernel32.dll"
            | b"bcrypt.dll" // TODO: check if this is a system library
            | b"ntdll.dll"
            | b"shell32.dll"
            | b"ole32.dll"
            | b"ws2_32.dll"
            | b"crypt32.dll"
            => {
                println!("system dll: {}", std::str::from_utf8(dll).unwrap());
                // known system library
            }
            unknown => {
                println!("ERROR: unknown dll: {:?}", std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8"));
                success = false;
            },
        }
    }

    success
}

fn process_elf_64<E : Endian>(binary: &[u8]) -> bool {
    use object::read::elf::*;
    use object::elf::*;

    let mut success = true;

    let parsed = ElfFile64::<E>::parse(binary).expect("failed to parse binary");

    for x in parsed.imports().unwrap() {
        println!("dynamic importing symbol: {}", std::str::from_utf8(x.name()).unwrap());
        success = false;
    }

    for segment in parsed.raw_segments() {
        if segment.p_type.get(parsed.endian()) == PT_INTERP {
            let data = segment.data(parsed.endian(), parsed.data()).unwrap();
            println!("interpreter: {:?}", std::str::from_utf8(data).unwrap());
            success = false;
        }
    }

    success
}
