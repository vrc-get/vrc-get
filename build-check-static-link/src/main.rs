use std::fs;
use object::{Endian, Endianness, FileKind};

fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let binary = args.next().unwrap();
    let binary = std::path::Path::new(&binary);
    let binary = fs::read(binary).unwrap();

    match FileKind::parse(binary.as_slice()).expect("detecting type") {
        FileKind::MachO64 => process_mach_64::<Endianness>(&binary),
        FileKind::Pe64 => process_pe_64(&binary),
        unknown => panic!("unknown file type: {:?}", unknown),
    }
}

fn process_mach_64<E : Endian>(binary: &[u8]) {
    use object::macho::*;
    use object::read::macho::*;

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
                if load_lc_str(data.name, command, endian) != b"/usr/lib/dyld" {
                    panic!("dylinker is not /usr/lib/dyld");
                } else {
                    println!("dylinker: /usr/lib/dyld");
                }
            },
            LC_LOAD_DYLIB => {
                let data = command.dylib().expect("parse LC_LOAD_DYLIB").unwrap();
                let dylib = load_lc_str(data.dylib.name, command, endian);
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
                    unknown => panic!("unknown dylib: {:?}", std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8")),
                }
            },
            unknown => panic!("unknown linker command: {unknown:08x}"),
        }
    }

    fn load_lc_str<'data, E : Endian>(s: LcStr<E>, d: LoadCommandData<'data, E>, endian: E) -> &'data [u8] {
        let offset = s.offset.get(endian);
        let bytes = &d.raw_data()[(offset as usize)..];
        let end_idx = bytes.iter().position(|x| x == &b'\0').unwrap_or(bytes.len());
        &bytes[..end_idx]
    }
}

fn process_pe_64(binary: &[u8]) {
    use object::read::pe::*;
    use object::LittleEndian as LE;

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
            unknown => panic!("unknown dll: {:?}", std::str::from_utf8(unknown).unwrap_or("unable to parse with utf8")),
        }
    }
}
