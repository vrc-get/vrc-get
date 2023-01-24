`vrc-get`
====

Open Source command line client of VRChat Package Manager. 

## Goals

- Provide Open Source command line client of VRChat Package Manager.
- Provide more functionality than official vpm command.

## Commands

For more details, please see --help

- [x] `vrc-get install [pkg] [version]`
- [x] `vrc-get remove [pkg]` (with alias `vrc-get rm [pkg]`)
- [x] `vrc-get repo list`
- [x] `vrc-get repo add <url> [NAME]`
- [x] `vrc-get repo remove <name or url>`
- [x] `vrc-get repo cleanup`
- [x] `vrc-get repo packages <NAME|url>`

# Installation

## Using `cargo`

If you have [`cargo`][cargo], installing using cargo is the easiest way to install vrc-get.

```bash
cargo install --locked --git https://github.com/anatawa12/vrc-get.git --tag <version>
```

## Prebuilt binaries

There's prebuilt binaries on the [release] page.

Download executable file for your platform and place to everywhere you want.
There's no additional requirements for thise binaries. All binaries are compiled statically as possible.

For linux, it's a actually static linked executable.

For windows, only `bcrypt.dll`, `ntdll.dll`, `kernel32.dll`, `advapi32.dll`, `ws2_32.dll`, `secur32.dll`, 
and `crypt32.dll`, which is builtin dlls, are dynamically linked.

For macOS, only `Security.framework`, `CoreFoundation.framework`, `libiconv.2.dylibs`, and `libSystem.B.dylibs`, 
which are macOS-builtin dylibs, are dynamically linked.

[cargo]: https://github.com/rust-lang/cargo/
[release]: https://github.com/anatawa12/vrc-get/releases/latest
