`vrc-get`
====

[![GitHub deployments][shields-github-deploy]][release-vrc-get]
[![crates.io deployments][shields-crates-io-deploy]][crates-io-vrc-get]
[![Github latest][shields-github-version]][release-vrc-get]
[![crates.io latest][shields-crates-io-version]][crates-io-vrc-get]
[![Homebrew Version][shields-homebrew-version]][homebrew-vrc-get]
[![MacPorts Version][shields-macports-vrc-get]][macports-vrc-get]
[![Scoop Version][shields-scoop-version]][scoop-vrc-get]
[![AUR Version][shields-aur-version]][aur-vrc-get]
[![WinGet Version][shields-winget-version]][winget-vrc-get]

Open Source command line client of VRChat Package Manager, 
the main feature of VRChat Creator Companion (VCC), which supports Windows, Linux, and macOS.

<small>This project is developed by community, not by VRChat.</small>

## Goals

### Initial goals

- Provide an open source command line client of VRChat Package Manager.
- Provide more functionality for VPM than official vpm commands can do.

### Additional goals since 2024
- Provide a better cross-platform Creator Companion

## Commands

For more details, please see --help

- [x] `vrc-get install [pkg] [version]` (with alias `vrc-get i [pkg] [version]` and `vrc-get resolve`) 
  install package to your project
- [x] `vrc-get remove [pkg]` (with alias `vrc-get rm [pkg]`) remove package from your project
- [x] `vrc-get outdated` list outdated packages in your project
- [x] `vrc-get upgrade [pkg] [version]` upgrade package(s) in your project
- [x] `vrc-get search <query...>` search package in installed repositories in your PC
- [x] `vrc-get repo list` list installed repositories in your PC
- [x] `vrc-get repo add <url> [NAME]` add repository into your PC
- [x] `vrc-get repo remove <name or url>` remove repository from your PC
- [x] `vrc-get repo cleanup` cleanup repo cache
- [x] `vrc-get repo packages <NAME|url>` list packages in specified repository

## Installation

### Using homebrew

If you're using [Homebrew], you can easily install vrc-get.

```
brew install vrc-get
```

### Using MacPorts

If you're using [MacPorts], you can easily install vrc-get with MacPorts.

```
sudo port install vrc-get
```

### Using winget

If you're using modern Windows, you can install vrc-get with [winget].

```pwsh
winget install anatawa12.vrc-get
```

### Using scoop

<!-- TODO: update when published to official repository -->

If you're using [scoop], you can install vrc-get from a scoop bucket.

```
scoop bucket add xrtools "https://github.com/babo4d/scoop-xrtools"
scoop install vrc-get
```

### Using PKGBUILD from AUR

If you're using Arch Linux, you can install vrc-get from Arch User Repository.
Please use your favorite AUR helpers.

```
yay -S vrc-get
```

### Using `cargo binstall`

If you have [`cargo binstall`][cargo-binstall], installing with cargo binstall is an easy & fast way to install vrc-get.
Cargo binstall will download & install prebuilt vrc-get from GitHub.

```bash
cargo binstall vrc-get
```

### Using `cargo`

If you have [cargo], installing with cargo is an easy way to install vrc-get.

```bash
cargo install vrc-get
```

### Prebuilt binaries

There are prebuilt binaries on the [release][release-vrc-get] page.

Download the executable file for your platform and place it anywhere you want.
There are no additional requirements for these binaries. All binaries are compiled statically as possible.

For Linux, it's actually a static linked executable.

For Windows, only `bcrypt.dll`, `ntdll.dll`, `kernel32.dll`, `advapi32.dll`, `ws2_32.dll`, `secur32.dll`, 
and `crypt32.dll`, which is builtin dlls, are dynamically linked.

For macOS, only `Security.framework`, `CoreFoundation.framework`, `libiconv.2.dylibs`, and `libSystem.B.dylibs`, 
which are macOS-builtin dylibs, are dynamically linked.

### For GitHub Actions

To use this tool to resolve(install) vpm dependencies, you have to use 
[`anatawa12/sh-actions/resolve-vpm-packages@master`][resolve-vpm-packages].

To use other commands, you have to install `vpm-get` via [`anatawa12/sh-actions/setup-vrc-get@master`][setup-vrc-get].

For more details, please refer to README for each action.

## GUI version of vrc-get

Since late February 2024, an experimental gui version of vrc-get has been available.
See [README of ALCOM][alcom] for more details.

[shields-github-deploy]: https://img.shields.io/github/deployments/anatawa12/vrc-get/master%20branch?label=github%20deployment
[shields-crates-io-deploy]: https://img.shields.io/github/deployments/anatawa12/vrc-get/crates.io?label=crates.io%20deployment
[shields-github-version]: https://img.shields.io/github/v/release/anatawa12/vrc-get
[shields-crates-io-version]: https://img.shields.io/crates/v/vrc-get
[shields-aur-version]: https://img.shields.io/aur/version/vrc-get
[shields-homebrew-version]: https://img.shields.io/homebrew/v/vrc-get
[shields-macports-vrc-get]: https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fports.macports.org%2Fapi%2Fv1%2Fports%2Fvrc-get%2F&query=%24.version&label=macports
[shields-scoop-version]: https://img.shields.io/scoop/v/vrc-get?bucket=https%3A%2F%2Fgithub.com%2Fbabo4d%2Fscoop-xrtools
[shields-winget-version]: https://img.shields.io/winget/v/anatawa12.vrc-get

<!-- TODO: macports: https://github.com/badges/shields/issues/9588 -->

[cargo]: https://github.com/rust-lang/cargo/
[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall#cargo-binaryinstall
[Homebrew]: https://brew.sh
[MacPorts]: https://www.macports.org
[scoop]: https://scoop.sh
[winget]: https://learn.microsoft.com/windows/package-manager/

[alcom]: ./vrc-get-gui/README.md

[release-vrc-get]: https://github.com/anatawa12/vrc-get/releases/latest
[resolve-vpm-packages]: https://github.com/anatawa12/sh-actions/tree/master/resolve-vpm-packages
[setup-vrc-get]: https://github.com/anatawa12/sh-actions/tree/master/setup-vrc-get

[crates-io-vrc-get]: https://crates.io/crates/vrc-get
[aur-vrc-get]: https://aur.archlinux.org/packages/vrc-get
[homebrew-vrc-get]: https://formulae.brew.sh/formula/vrc-get
[macports-vrc-get]: https://ports.macports.org/port/vrc-get
[scoop-vrc-get]: https://github.com/babo4d/scoop-xrtools/blob/master/bucket/vrc-get.json
[winget-vrc-get]: https://github.com/microsoft/winget-pkgs/tree/master/manifests/a/anatawa12/vrc-get

## Contribution

- For how to contribute vrc-get: [CONTRIBUTING.md](CONTRIBUTING.md)
- For how to contribute localization to ALCOM (vrc-get-gui): [vrc-get-gui/CONTRIBUTING.md](vrc-get-gui/CONTRIBUTING.md) (**Please read [CONTRIBUTING.md#configuration-requirements](CONTRIBUTING.md#configuration-requirements) first before you read [vrc-get-gui/CONTRIBUTING.md](vrc-get-gui/CONTRIBUTING.md)!**)
