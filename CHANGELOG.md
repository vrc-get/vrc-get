# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [1.9.1] - 2025-07-28
### Changed
- Changed how we read VCC's project information [`#1997`](https://github.com/vrc-get/vrc-get/pull/1997)
  - Along with this, building this project no longer needs dotnet SDK to build.
- Migrated the project to Rust 2024 [`#1956`](https://github.com/vrc-get/vrc-get/pull/1956)
  - This is internal changes should not cause behavior changes
  - This would require Rust 1.85 for building this project
- Removed `cargo-about` from build-time dependency [`#1961`](https://github.com/vrc-get/vrc-get/pull/1961)
  - This is internal changes should not cause behavior changes
  - I listed here since this may need update on package metadata of some package managers
- The method to retrieve the list of Unity from Unity Hub [`#1808`](https://github.com/vrc-get/vrc-get/pull/1808) [`#1971`](https://github.com/vrc-get/vrc-get/pull/1971)
- You now can select multiple folders at once to adding project [`#2018`](https://github.com/vrc-get/vrc-get/pull/2018)
  - I didn't know official VCC had such a feature. Sorry for lack of feature!
- The requirements for unity project [`#2106`](https://github.com/vrc-get/vrc-get/pull/2106)
  - Since this version, `Projectsettings/ProjectVersion.txt` is required.

### Fixed
- Uninstall package is not reverted successfully if removing package is prevented by `ERROR_SHARING_VIOLATION` [`#2209`](https://github.com/vrc-get/vrc-get/pull/2209)
- Fixed `a - b` version range is not correctly serialized on the `vpm-manifest.json`

## [1.9.0] - 2025-01-01
### Added
- Per-package `headers` field support [`#718`](https://github.com/vrc-get/vrc-get/pull/718)
  - Since this is adding support for missing features, I treat this as a bugfix and not bump minor version.
- De-duplicating duplicated projects or Unity in VCC project list [`#1081`](https://github.com/vrc-get/vrc-get/pull/1081)
- `vrc-get cache clear`, command to clear package cache [`#1204`](https://github.com/vrc-get/vrc-get/pull/1204)
- Importing / Exporting Repositories list [`#1209`](https://github.com/vrc-get/vrc-get/pull/1209)
  - `vrc-get repo import <list file>` and `vrc-get repo export`
- User Package Management [`#1222`](https://github.com/vrc-get/vrc-get/pull/1222)
  - This release adds `vrc-get user-package` subcommands to manage user packages.
- `vrc-get reinstall <package id>` to reinstall specified packages [`#1223`](https://github.com/vrc-get/vrc-get/pull/1223)
- Skipping finding legacy assets when downgrading / upgrading / reinstalling package [`#1581`](https://github.com/vrc-get/vrc-get/pull/1581)
  - This will speed up the process of downgrading / upgrading / reinstalling package.

### Changed
- Error message will be shown if the SHA256 hash of the downloaded zip file does not match with the hash in the repository [`#1183`](https://github.com/vrc-get/vrc-get/pull/1183)
  - Currently, official VCC does not verify the hash of the downloaded zip file, but it's better to verify the hash.
  - For compatibility, even if the hash does not match, the file will be extracted with an error message.
  - In the future, we may make this a hard error.
- Migration feature is no longer marked as experimental [`#1559`](https://github.com/vrc-get/vrc-get/pull/1559)

### Fixed
- Unity from Unity Hub will be registered as manually registered Unity [`#1081`](https://github.com/vrc-get/vrc-get/pull/1081)
- Fails to uninstall packages on macOS with filesystem that doesn't support resource fork [`#1402`](https://github.com/vrc-get/vrc-get/pull/1402)
  - This is typically seen on ExFAT or FAT32 filesystems, not on APFS or HFS+ filesystems.
  - macOS internally creates files starting with `._` for resource fork if the filesystem does not support resource fork.
  - vrc-get-vpm does not handle this file correctly and fails to uninstall the package.
- Prerelease version is choosen even if good stable version exists [`#1745`](https://github.com/vrc-get/vrc-get/pull/1745)

## [1.8.2] - 2024-10-16
### Fixed
- Hotfix: Added contact information about author of the project to the User-Agent

## [1.8.1] - 2024-05-13
### Changed
- Relax validation for `package.json` for local user packages [`#750`](https://github.com/vrc-get/vrc-get/pull/750)
- Improved handling for unlocked packages [`#790`](https://github.com/vrc-get/vrc-get/pull/790)
- Legacy Assets are remove even if the specified GUID does not match with the actual GUID [`#901`](https://github.com/vrc-get/vrc-get/pull/901)
  - This follows VCC 2.3.0 beta 3 behavior.
- Updated the recommended Unity 2022 version to 2022.3.22f1 [`#928`](https://github.com/vrc-get/vrc-get/pull/928)
- Support repositories with bad packages [`#954`](https://github.com/vrc-get/vrc-get/pull/954)
- Use url in settings.json to load remote repository [`#955`](https://github.com/vrc-get/vrc-get/pull/955)

### Fixed
- Installing package while Unity can be failed [`#696`](https://github.com/vrc-get/vrc-get/pull/696)

## [1.8.0] - 2024-03-31
### Added
- Command to migrate a legacy VRCSDK3 project to VPM VRCSDK [`#580`](https://github.com/vrc-get/vrc-get/pull/580)
  - You can do with `vrc-get migrate vpm` command.

### Changed
- vrc-get now finds unity hub from registry key [`#590`](https://github.com/vrc-get/vrc-get/pull/590)
- vrc-get specific configuration is moved to `vrc-get/settings.json` [`#607`](https://github.com/vrc-get/vrc-get/pull/607)
  - This is done because we separated file for vrc-get-gui config file, and we may have more in the future os
    to not dirty the folder, I moved the config file to `vrc-get` folder.
- Legacy Assets are not removed if the specified GUID does not match with the actual GUID [`#677`](https://github.com/vrc-get/vrc-get/pull/677)
  - This follows VCC 2.3.0 beta behavior.

### Fixed
- Some embedded / installed vpm package is not recognized by vrc-get [`#609`](https://github.com/vrc-get/vrc-get/pull/609)
  - This makes `d4rkAvatarOptimizer` is recognized as not installed.
- Fails to load all repositories if fails to load one repository [`#551`](https://github.com/vrc-get/vrc-get/pull/551)
- Legacy Projects cannot be added to VCC project list [`#586`](https://github.com/vrc-get/vrc-get/pull/586)
- Bad behaviors with unity 2018 [`#605`](https://github.com/vrc-get/vrc-get/pull/605)
- Prerelease packages are installed if the version range contains prerelease [`#676`](https://github.com/vrc-get/vrc-get/pull/676)
  - To fix this problem, automatic allow prerelease rule is a bit changed.
  - For resolving dependencies in `vpm-manifest.json`, prerelease is used if version range contains prerelease.
  - For resolving dependencies of packages, prerelease is used if the dependant package is prerelease and the matching version is only contained in prereleases.
  - This does not change behavior of `--prerelease` option.
- False positive conflicts with legacy packages [`#676`](https://github.com/vrc-get/vrc-get/pull/676)

## [1.7.1] - 2024-03-01
### Changed
- When you call `vrc-get vcc` without enabling `experimental-vcc` feature, it will show you a warning [`#485`](https://github.com/anatawa12/vrc-get/pull/485)

### Fixed
- Empty `depeneencies` block of `locked` section in `vpm-manifest.json` is removed [`#478`](https://github.com/anatawa12/vrc-get/pull/478)
  - This follows the changed behavior of the official VPM command.

## [1.7.0] - 2024-02-27
### Added
- `vrc-get downgrade` which is for downgrading package [`#409`](https://github.com/anatawa12/vrc-get/pull/409)

### Changed
- `vrc-get` will not search `Packages` folder for legacy assets with GUID [`#439`](https://github.com/anatawa12/vrc-get/pull/439)
  - Specifying `Packages/<package id>` folder as a legacy folder is still supported.

### Fixed
- projects migrated from `settings.json` to litedb does not have `UnityVersion` [`#411`](https://github.com/anatawa12/vrc-get/pull/411)
- `vrc-get repo add` adds a relative path to `settings.json` [`#440`](https://github.com/anatawa12/vrc-get/pull/440)
- `vrc-get repo add` accepts invalid path to the local repository [`#440`](https://github.com/anatawa12/vrc-get/pull/440)
- last modified on the vcc project database is not updated [`#443`](https://github.com/anatawa12/vrc-get/pull/443)

## [1.6.1] - 2024-02-25
### Fixed
- repositories with `vrc-get.yank` but without `vrc-get.aliases` not working

## [1.6.0] - 2024-02-16
### Added
- Changelog [`#351`](https://github.com/anatawa12/vrc-get/pull/351)
  - I wrote changelog for all releases for all releases
- global (whole-user) configuration for vrc-get [`#352`](https://github.com/anatawa12/vrc-get/pull/352)
  - create in the `%LOCALAPPDATA%/VRChatCreatorCompanion/vrc-get-settings.json` or `$XDG_DATA_HOME/VRChatCreatorCompanion/vrc-get-settings.json`
  - This feature is not stable yet. Using this feature will warn you about it and use at your own risk. [`#362`](https://github.com/anatawa12/vrc-get/pull/362)
- Feature to ignore official / curated repositories [`#352`](https://github.com/anatawa12/vrc-get/pull/352)
  - You can enable this feature in `vrc-get-settings.json` by setting `ignoreOfficialRepository` or `ignoreCuratedRepository` to true.
  - This feature is replacement of `experimental-override-predefined` feature since 1.4.1.
    - Please add alternative repository to user repository and enable this feature to use alternative repository instead of official / curated repository.
  - This feature is not stable yet. Using this feature will warn you about it and use at your own risk. [`#362`](https://github.com/anatawa12/vrc-get/pull/362) [`#365`](https://github.com/anatawa12/vrc-get/pull/365)
- `vrc-get-litedb` crate which is NativeAOT based LiteDB wrapper for vrc-get [`#320`](https://github.com/anatawa12/vrc-get/pull/320)
- `vrc-get vcc` commands which is a command for vrc-get as a VCC project [`#369`](https://github.com/anatawa12/vrc-get/pull/369) [`#396`](https://github.com/anatawa12/vrc-get/pull/396)
  - This feature is disabled by default. pass `--features experimental-vcc` to cargo to enable this feature. [`#384`](https://github.com/anatawa12/vrc-get/pull/384) 
  - `vrc-get vcc project list` to list projects [`#369`](https://github.com/anatawa12/vrc-get/pull/369)
  - `vrc-get vcc project add <path>` to add a project to project list [`#369`](https://github.com/anatawa12/vrc-get/pull/369)
  - `vrc-get vcc project remove <path>` to remove a project from project list [`#369`](https://github.com/anatawa12/vrc-get/pull/369)
  - `vrc-get vcc unity list` to list unity in vcc's unity list [`#382`](https://github.com/anatawa12/vrc-get/pull/382)
  - `vrc-get vcc unity add <path>` to add a unity to vcc's unity list [`#382`](https://github.com/anatawa12/vrc-get/pull/382)
  - `vrc-get vcc unity remove <path>` to remove a unity from vcc's unity list [`#382`](https://github.com/anatawa12/vrc-get/pull/382)
  - In addition, `vrc-get migrate unity 2022` command will get unity from the vcc's unity list [`#382`](https://github.com/anatawa12/vrc-get/pull/382)
- Tests for `vrc-get-vpm` module. [`#393`](https://github.com/anatawa12/vrc-get/pull/393)
  - For basic project operations, I added tests in `vrc-get-vpm` module.
- `vrc-get i --name <name>` to install package by name [`#398`](https://github.com/anatawa12/vrc-get/pull/398)
  - Currently, name-based search ignores whitespace in the name.
  - This feature is experimental. Behavior may change in the future.
- `vrc-get` property in `package.json` for extra metadata for vrc-get [`#399`](https://github.com/anatawa12/vrc-get/pull/399)
  - with `yanked` field, you can yank the package from the repository. [`#399`](https://github.com/anatawa12/vrc-get/pull/399)
    - setting `"yanked": true` will make your package yanked and `"yanked": "reason"` tell the reason of yanking.
  - with `aliases` field, you can add aliases for `vrc-get i --name` described above [`#399`](https://github.com/anatawa12/vrc-get/pull/399)
    - since `vrc-get i --name` is experimental, this feature is also experimental.
- `zipSHA256` property support [`#406`](https://github.com/anatawa12/vrc-get/pull/406)
  - You can add `zipSHA256` property to `package.json` to specify SHA256 hash of the zip file.
  - Currently `vrc-get` verifies the hash of the zip file when using cache as VCC does.
  - I may add verification for downloaded zip file in the future.
- Better logging style [`#407`](https://github.com/anatawa12/vrc-get/pull/407)
  - Now, `vrc-get` uses our formatting style for logging if none of `RUST_LOG_STYLE` and `RUST_LOG` are set.
  - This style is shorter than `env_logger`'s default style so better for default CLI usage.
  - If you need more detailed logs, you can set `RUST_LOG` to get more detailed information.

### Changed
- Improved confirmation behaviour about updating `dependencies` [`#395`](https://github.com/anatawa12/vrc-get/pull/395)
  - Previously, the confirmation show nothing about updating `dependencies` since it's almost safe (just updating `vpm-manifest.json`).
  - Since this release, `vrc-get` shows about what's updating `dependencies`
  - In addition, if actual operaion is just updating `dependencies`, `vrc-get` will just show updates and apply changes without confirmation.

### Fixed
- Warnings about index map [`#350`](https://github.com/anatawa12/vrc-get/pull/350)
- `vrc-get repo remove` not working [`#361`](https://github.com/anatawa12/vrc-get/pull/361)
- `writing local repo cache 'Repos/vrc-curated.json'` error [`#365`](https://github.com/anatawa12/vrc-get/pull/365)
- Prompt is shown even if there is nothing to update [`#393`](https://github.com/anatawa12/vrc-get/pull/393)
- Conflict warning can be shown even if new conflicts are not caused [`#393`](https://github.com/anatawa12/vrc-get/pull/393) [`#400`](https://github.com/anatawa12/vrc-get/pull/400)
- Directory traversal with `legacyFolders` and `legacyFiles` [`#393`](https://github.com/anatawa12/vrc-get/pull/393)
- You can add unused package to locked with `vrc-get upgrade` [`#403`](https://github.com/anatawa12/vrc-get/pull/403)
  - Now, vrc-get show you `Package <id> is not locked, so it cannot be upgraded` error.
- Panic when upgrading unused package with `vrc-get upgrade` [`#403`](https://github.com/anatawa12/vrc-get/pull/403)

## [1.5.3] - 2024-02-03
### Fixed
- Partial version ends with `-` is not supported [`#335`](https://github.com/anatawa12/vrc-get/pull/335)
- LF is used for line separator on windows [`#343`](https://github.com/anatawa12/vrc-get/pull/343)

## [1.5.2] - 2024-01-21
### Fixed
- UPM manifest without dependencies block [`#331`](https://github.com/anatawa12/vrc-get/pull/331)

### Security
- DoS vulnerability in `h2` crate is fixed in this release [`#332`](https://github.com/anatawa12/vrc-get/pull/332)
  - See [GHSA-8r5v-vm4m-4g25](https://github.com/advisories/GHSA-8r5v-vm4m-4g25) for more information about this vulnerability

## [1.5.1] - 2024-01-16
### Added
- More goals to the README [`#327`](https://github.com/anatawa12/vrc-get/pull/327)

### Fixed
- Prebuilt binaries are not static linked [`#329`](https://github.com/anatawa12/vrc-get/pull/329)

## [1.5.0] - 2024-01-14
### Added
- `vrc-get migrate unity project 2022` to migrate a project to Unity 2022 [`#309`](https://github.com/anatawa12/vrc-get/pull/309)
- Support for `vrc-get resolve` to resolve a template project [`#310`](https://github.com/anatawa12/vrc-get/pull/310)

### Fixed
- `vrc-get rm <package>` cannot remove legacy package of install packages [`#312`](https://github.com/anatawa12/vrc-get/pull/312)

## [1.4.3] - 2024-01-06
### Changed
- Internally merged resolve, install, update, and remove process [`#299`](https://github.com/anatawa12/vrc-get/pull/299)
- `vrc-get resolve <package> <version>` and `vrc-get resolve --prerelease` is now hard error. It's unexpectedly accepted. [`#299`](https://github.com/anatawa12/vrc-get/pull/299)
- `vrc-get remove` now confirms if there is conflict [`#299`](https://github.com/anatawa12/vrc-get/pull/299)
- Enabled cargo distribution since deflate64 support of async_zip is now published to crates.io [`#300`](https://github.com/anatawa12/vrc-get/pull/300) [`#301`](https://github.com/anatawa12/vrc-get/pull/301)

## [1.4.2] - 2023-12-24
### Fixed
- `vrc-get upgrade` all packages is broken [`#287`](https://github.com/anatawa12/vrc-get/pull/287)

## [1.4.1] - 2023-12-23
### Added
- Experimental yank support [`#273`](https://github.com/anatawa12/vrc-get/pull/273)
- Experimental support for overriding official repository with another repository [`#274`](https://github.com/anatawa12/vrc-get/pull/274)
  - Those experimental features are under feature flags and not enabled by default [`#285`](https://github.com/anatawa12/vrc-get/pull/285)

### Changed
- Changed to our own semver impl since `semver` crate does not for npm-style version range [`#270`](https://github.com/anatawa12/vrc-get/pull/270) [`#277`](https://github.com/anatawa12/vrc-get/pull/277)
- Internally separated crate for `vrc-get` vpm client core [`#271`](https://github.com/anatawa12/vrc-get/pull/271)
- `vrc-get resolve` now reports installed packages [`#273`](https://github.com/anatawa12/vrc-get/pull/273)

### Fixed
- Some process are not in parallel [`#278`](https://github.com/anatawa12/vrc-get/pull/278)
- Resolving prerelease packages from unlocked packages fails [`#284`](https://github.com/anatawa12/vrc-get/pull/284)

## [1.4.0] - 2023-12-10
### Added
- `unity` field on package json support [`#264`](https://github.com/anatawa12/vrc-get/pull/264)
  - This includes Unity 2022 / VCC 2.2.0 support

## [1.3.2] - 2023-10-15
### Added
- Scoop installation to README [`#233`](https://github.com/anatawa12/vrc-get/pull/233)

### Fixed
- Possible infinity loop with deflate64 compression [`#241`](https://github.com/anatawa12/vrc-get/pull/241)

## [1.3.1] - 2023-10-06
### Changed
- Now we use `rustls` as a default TLS backend instead of `native-tls` [`#232`](https://github.com/anatawa12/vrc-get/pull/232)

## [1.3.0] - 2023-09-21
### Added
- `legacyPackages` support [`#219`](https://github.com/anatawa12/vrc-get/pull/219)
- version range notion in dependencies block [`#221`](https://github.com/anatawa12/vrc-get/pull/221)

### Fixed
- Several UI problems [`#222`](https://github.com/anatawa12/vrc-get/pull/222)

## [1.2.0] - 2023-09-17
### Changed
- `vrc-get info project` now shows installed package information in human-readable format [`518e8c3`](https://github.com/anatawa12/vrc-get/commit/518e8c3232ff2f42822819e85f5c961a6793a208)
- `async_zip` is now as a git dependencies instead of submodule [`4be0b16`](https://github.com/anatawa12/vrc-get/commit/4be0b1610c37787dc3886395dc43061c3fcb4a77)

### Fixed
- There are no information on error with unlocked package [`2814626`](https://github.com/anatawa12/vrc-get/commit/2814626d3615f8478d811383b1431aec07bca220)

## [1.1.3] - 2023-08-20
### Added
- `completion` support [`#36`](https://github.com/anatawa12/vrc-get/pull/36)

### Changed
- Temporary removed crates.io distribution since deflate64 compression is not supported by packages on crates.io

### Fixed
- zip file with deflate64 compression cannot be extracted [`#200`](https://github.com/anatawa12/vrc-get/pull/20)

## [1.1.2] - 2023-07-08
### Fixed
- Conflict error if the locked version is older than dependencies version [`#177`](https://github.com/anatawa12/vrc-get/pull/177)

## [1.1.1] - 2023-06-25
### Fixed
- Error with BOM from remote repository [`#167`](https://github.com/anatawa12/vrc-get/pull/167)

## [1.1.0] - 2023-06-19
### Added
- `vrc-get info project` now shows information about dependencies [`#165`](https://github.com/anatawa12/vrc-get/pull/165)  

### Fixed
- Auto removing unused packages does not consider unlocked packages [`#160`](https://github.com/anatawa12/vrc-get/pull/160)

## [1.0.2] - 2023-06-07
### Added
- `vrc-get update` to update all local repository cache [`#154`](https://github.com/anatawa12/vrc-get/pull/154)
- `--no-update` option not to update all local repository cache [`#154`](https://github.com/anatawa12/vrc-get/pull/154)
- `vrc-get info project` to get installed package information [`#154`](https://github.com/anatawa12/vrc-get/pull/154)
- `vrc-get info package` to package information [`#154`](https://github.com/anatawa12/vrc-get/pull/154)

## [1.0.1] - 2023-05-15
### Fixed
- Error with null guid on `legacyFolders` or `legacyFolders` [`#146`](https://github.com/anatawa12/vrc-get/pull/146) 
- Legacy assets are removed if installing package failed during installing [`#147`](https://github.com/anatawa12/vrc-get/pull/147)

## [1.0.0] - 2023-04-30
### Fixed
- `1.0.x` does not match `1.0.0-beta` [`#135`](https://github.com/anatawa12/vrc-get/pull/135)

## [0.2.6] - 2023-04-27
### Fixed
- versions on `vrc-get repo packages` is not sorted [`#128`](https://github.com/anatawa12/vrc-get/pull/128)
- `1.0.x` does not match `1.0.0-beta` [`#133`](https://github.com/anatawa12/vrc-get/pull/133)

## [0.2.5] - 2023-04-16
### Changed
- improved support for prerelease packages [`#126`](https://github.com/anatawa12/vrc-get/pull/126)

### Fixed
- Prompting breaks the cursor [`#127`](https://github.com/anatawa12/vrc-get/pull/127)

## [0.2.4] - 2023-04-15
### Fixed
- Repositories without `id` will be removed from `settings.json` [`#125`](https://github.com/anatawa12/vrc-get/pull/125)

## [0.2.3] - 2023-04-15
### Added
- Removes id duplicated repositories after mutating operation [`#123`](https://github.com/anatawa12/vrc-get/pull/123)

### Changed
- Now prebuilt `vrc-get` for windows is almost static linked [`#121`](https://github.com/anatawa12/vrc-get/pull/121)
  - All dynamically linked dlls are NT-kernel dlls so it's no longer needed to install any dlls to run prebuilt `vrc-get` on windows

### Removed
- Support for repositories not listed on `settings.json` [`#122`](https://github.com/anatawa12/vrc-get/pull/122)

### Fixed
- bad behaviors with `package-cache.json` [`#119`](https://github.com/anatawa12/vrc-get/pull/119)
- Error with local repository cahce without `headers` [`#120`](https://github.com/anatawa12/vrc-get/pull/120)

## [0.2.2] - 2023-04-09
### Added
- `vrc-get repo add -H` as alias of `vrc-get add repo --headers` [`#113`](https://github.com/anatawa12/vrc-get/pull/113)
- `vrc-get repo rm` now accepts repository id [`#113`](https://github.com/anatawa12/vrc-get/pull/113)

### Changed
- `vrc-get repo add` now creates local cache with `id` as a file name [`#112`](https://github.com/anatawa12/vrc-get/pull/112)

## [0.2.1] - 2023-04-09
### Fixed
- Several bugs about `id` and `headers` [`#105`](https://github.com/anatawa12/vrc-get/pull/105)

## [0.2.0] - 2023-04-07
### Added
- Support for legacyFolders [`#99`](https://github.com/anatawa12/vrc-get/pull/99)
- Confirm before adding packages [`#99`](https://github.com/anatawa12/vrc-get/pull/99)
- Support for `id` field of repository [`#89`](https://github.com/anatawa12/vrc-get/pull/89) [`#100`](https://github.com/anatawa12/vrc-get/pull/100)
- Adding repositories with `headers` configuration of repository [`#102`](https://github.com/anatawa12/vrc-get/pull/102)

### Changed
- Improved dependency resolution algorithm [`#91`](https://github.com/anatawa12/vrc-get/pull/91)
- Internally split process to fetch remote repository and resolving packages [`#97`](https://github.com/anatawa12/vrc-get/pull/97)

### Fixed
- User packages are not proceed [`#98`](https://github.com/anatawa12/vrc-get/pull/98)

## [0.1.13] - 2023-03-28
### Changed
- Updated multiple dependencies

### Fixed
- Multiple requests are made to the same Repository [`#77`](https://github.com/anatawa12/vrc-get/pull/77)
- Release action is still broken [`#78`](https://github.com/anatawa12/vrc-get/pull/78)

## [0.1.12] - 2023-03-22
### Security
- Possible directory traversal [`#71`](https://github.com/anatawa12/vrc-get/pull/71) [`#72`](https://github.com/anatawa12/vrc-get/pull/72)

## [0.1.11] - 2023-03-21
### Fixed
- Release action is still broken [`#65`](https://github.com/anatawa12/vrc-get/pull/65)
- Possible infinity loop when installing package [`#67`](https://github.com/anatawa12/vrc-get/pull/67)

### Security
- Possible directory traversal [`#68`](https://github.com/anatawa12/vrc-get/pull/68)

## [0.1.10] - 2023-03-11
### Added
- Small description for each command on the README [`#47`](https://github.com/anatawa12/vrc-get/pull/47)
- More debug-level logs [`#56`](https://github.com/anatawa12/vrc-get/pull/56)
- Improved way to get LocalAppData folder path [`#57`](https://github.com/anatawa12/vrc-get/pull/57)

### Fixed
- Release action is still broken [`#48`](https://github.com/anatawa12/vrc-get/pull/48)
- README [`#50`](https://github.com/anatawa12/vrc-get/pull/50) [`#52`](https://github.com/anatawa12/vrc-get/pull/52)
- `vrc-get add repo` is not working [`#64`](https://github.com/anatawa12/vrc-get/pull/64)

## [0.1.9] - 2023-02-16
### Fixed
- Unlocked packages are not proceeded correctly on resolve command [`#45`](https://github.com/anatawa12/vrc-get/pull/45)
- Releasing to homebrew is not working [`#46`](https://github.com/anatawa12/vrc-get/pull/46)

## [0.1.8] - 2023-02-14
### Added
- Automatic distribution to homebrew [`#37`](https://github.com/anatawa12/vrc-get/pull/37)
- Support for non-locked but exists packages [`#42`](https://github.com/anatawa12/vrc-get/pull/42) [`#43`](https://github.com/anatawa12/vrc-get/pull/43)

### Changed
- Improved README [`#38`](https://github.com/anatawa12/vrc-get/pull/38)

### Fixed
- Error occurs with bom in the file [`#40`](https://github.com/anatawa12/vrc-get/pull/40)

## [0.1.7] - 2023-02-10
### Added
- Commands added in 0.1.6 to readme [`#31`](https://github.com/anatawa12/vrc-get/pull/31)
- `--offline` option to many commands [`#32`](https://github.com/anatawa12/vrc-get/pull/32)
- Link to homebrew distribution [`#34`](https://github.com/anatawa12/vrc-get/pull/34)

### Changed
- Improved error message for most commands [`#33`](https://github.com/anatawa12/vrc-get/pull/33)

## [0.1.6] - 2023-02-09
### Added
- Notice this is not VRChat-official project. [`f9c1635`](https://github.com/anatawa12/vrc-get/commit/f9c1635ae439250435d9b1d97a7e715247c2d4d0)
- `vrc-get search` [`#26`](https://github.com/anatawa12/vrc-get/pull/26)
- `vrc-get resolve` as a alias of `vrc-get install` [`#27`](https://github.com/anatawa12/vrc-get/pull/27)
- Crates.io distribution [`#28`](https://github.com/anatawa12/vrc-get/pull/28)

### Fixed
- Unused packages are not removed after updating / removing package [`#24`](https://github.com/anatawa12/vrc-get/pull/24)
- Rust cache is shared between platforms [`#30`](https://github.com/ananatawa12/vrc-get/pull/30)

## [0.1.5] - 2023-02-06
### Added
- Installation step for installation to GitHub actions [`9242a63`](https://github.com/anatawa12/vrc-get/commit/9242a632b1e817b8e5a3dd4a921c4f5a4f4e5dfb) [`be62276`](https://github.com/anatawa12/vrc-get/commit/be622766aa375231988e9b2ebeee7433349c9256)
- `vrc-get outdated` command to check if there are outdated packages [`17f900c`](https://github.com/anatawa12/vrc-get/commit/17f900cfb20267c0a26df78311ea781df1288348)
- `vrc-get update` command to update all/specified packages [`60d2dbf`](https://github.com/anatawa12/vrc-get/commit/60d2dbf0e20b0b16ec3675c479a933bba913693f)
- `vrc-get outdated --json-format 1` to get outdated packages in machine-readable JSON format [`99e82db`](https://github.com/anatawa12/vrc-get/commit/99e82dbe4a73f35ffa8e0185d3aa35ba8d336f20)

### Changed
- Reduced memory usage and improved speed of downloading package by using `stream` feature of `reqwest` [`fd1cf49`](https://github.com/anatawa12/vrc-get/commit/fd1cf4904138ee9951045b728456bd02c496351a)

## [0.1.4] - 2023-01-25
### Changed
- Speed of `vrc-get resolve` command [`dcc8561`](https://github.com/anatawa12/vrc-get/commit/dcc856136c8cfe9f37ef7504df3690ccd4d6a5ff)

### Fixed
- Error if repository cache folder does not exists [`4fe7d59`](https://github.com/anatawa12/vrc-get/commit/4fe7d5951d1d3bcd738c91621b5bd9b5121ae041)

## [0.1.3] - 2023-01-25
### Fixed
- Error occurs if global configuration folder does not exists [`0bf9b44`](https://github.com/anatawa12/vrc-get/commit/0bf9b44ff8e523fdcf786e00ca8bfa9551317464)

## [0.1.2] - 2023-01-25
### Added
- Documentation to install this tool [`89d404b`](https://github.com/anatawa12/vrc-get/commit/89d404bcb0ef8a40cd3dac750afaf96240975f4b)
- Check SHA256 hash of zip to check if the file is valid [`68442ad`](https://github.com/anatawa12/vrc-get/commit/68442add6b42c84a99d6a0248de7ac6acdf55b20)
- `vrc-get i` as alias of `vrc-get install` [`7df45e5`](https://github.com/anatawa12/vrc-get/commit/7df45e58788152c5f31cf493b051c2a6e91db1ca) [`a5abf45`](https://github.com/anatawa12/vrc-get/commit/a5abf45bbc0067d89599b05c837262a2ef3138c1)
- etag based cache for repository json [`ee59813`](https://github.com/anatawa12/vrc-get/commit/ee598138f48f56498ca90723ba467e21f02f94db)

### Fixed
- Release action is still broken
- `vrc-get rm` is not working [`46144fd`](https://github.com/anatawa12/vrc-get/commit/46144fdac52f6cbb9462912b1074d4d08a0bd431)
- `--version` is not working [`8c416dc`](https://github.com/anatawa12/vrc-get/commit/8c416dc6be7eea3c5d293bee0023d6b81d15c7db)
- `vrc-get install` installs the oldest version instead of newest version [`6f0cc7b`](https://github.com/anatawa12/vrc-get/commit/6f0cc7bd4107f827b4a2463bf199230962c935a7)
- `vrc-get install` does not add dependencies to `locked` [`f7b3511`](https://github.com/anatawa12/vrc-get/commit/f7b3511b0ea548074c5bc681b4f0dfdcde7d553d)

## [0.1.1] - 2023-01-25
### Fixed
- Release action is broken

## [0.1.0] - 2023-01-25
Initial Release

[Unreleased]: https://github.com/vrc-get/vrc-get/compare/v1.9.1...HEAD
[1.9.1]: https://github.com/vrc-get/vrc-get/compare/v1.9.0...v1.9.1
[1.9.0]: https://github.com/vrc-get/vrc-get/compare/v1.8.2...v1.9.0
[1.8.2]: https://github.com/vrc-get/vrc-get/compare/v1.8.1...v1.8.2
[1.8.1]: https://github.com/vrc-get/vrc-get/compare/v1.8.0...v1.8.1
[1.8.0]: https://github.com/vrc-get/vrc-get/compare/v1.7.1...v1.8.0
[1.7.1]: https://github.com/anatawa12/vrc-get/compare/v1.7.0...v1.7.1
[1.7.0]: https://github.com/anatawa12/vrc-get/compare/v1.6.1...v1.7.0
[1.6.1]: https://github.com/anatawa12/vrc-get/compare/v1.6.0...v1.6.1
[1.6.0]: https://github.com/anatawa12/vrc-get/compare/v1.5.3...v1.6.0
[1.5.3]: https://github.com/anatawa12/vrc-get/compare/v1.5.2...v1.5.3
[1.5.2]: https://github.com/anatawa12/vrc-get/compare/v1.5.1...v1.5.2
[1.5.1]: https://github.com/anatawa12/vrc-get/compare/v1.5.0...v1.5.1
[1.5.0]: https://github.com/anatawa12/vrc-get/compare/v1.4.3...v1.5.0
[1.4.3]: https://github.com/anatawa12/vrc-get/compare/v1.4.2...v1.4.3
[1.4.2]: https://github.com/anatawa12/vrc-get/compare/v1.4.1...v1.4.2
[1.4.1]: https://github.com/anatawa12/vrc-get/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/anatawa12/vrc-get/compare/v1.3.2...v1.4.0
[1.3.2]: https://github.com/anatawa12/vrc-get/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/anatawa12/vrc-get/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/anatawa12/vrc-get/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/anatawa12/vrc-get/compare/v1.1.3...v1.2.0
[1.1.3]: https://github.com/anatawa12/vrc-get/compare/v1.1.2...v1.1.3
[1.1.2]: https://github.com/anatawa12/vrc-get/compare/v1.1.1...v1.1.2
[1.1.1]: https://github.com/anatawa12/vrc-get/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/anatawa12/vrc-get/compare/v1.0.2...v1.1.0
[1.0.2]: https://github.com/anatawa12/vrc-get/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/anatawa12/vrc-get/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/anatawa12/vrc-get/compare/v0.2.6...v1.0.0
[0.2.6]: https://github.com/anatawa12/vrc-get/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/anatawa12/vrc-get/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/anatawa12/vrc-get/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/anatawa12/vrc-get/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/anatawa12/vrc-get/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/anatawa12/vrc-get/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/anatawa12/vrc-get/compare/v0.1.13...v0.2.0
[0.1.13]: https://github.com/anatawa12/vrc-get/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/anatawa12/vrc-get/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/anatawa12/vrc-get/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/anatawa12/vrc-get/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/anatawa12/vrc-get/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/anatawa12/vrc-get/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/anatawa12/vrc-get/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/anatawa12/vrc-get/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/anatawa12/vrc-get/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/anatawa12/vrc-get/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/anatawa12/vrc-get/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/anatawa12/vrc-get/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/anatawa12/vrc-get/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/anatawa12/vrc-get/releases/tag/v0.1.0
