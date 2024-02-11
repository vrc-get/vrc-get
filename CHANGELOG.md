# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog].

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/

## [Unreleased]
### Added
- Changelog `#351`
  - I wrote changelog for all releases for all releases
- global (whole-user) configuration for vrc-get `#352`
  - create in the `%LOCALAPPDATA%/VRChatCreatorCompanion/vrc-get-settings.json` or `$XDG_DATA_HOME/VRChatCreatorCompanion/vrc-get-settings.json`
  - This feature is not stable yet. Using this feature will warn you about it and use at your own risk. `#362`
- Feature to ignore official / curated repositories `#352`
  - You can enable this feature in `vrc-get-settings.json` by setting `ignoreOfficialRepository` or `ignoreCuratedRepository` to true.
  - This feature is replacement of `experimental-override-predefined` feature since 1.4.1.
    - Please add alternative repository to user repository and enable this feature to use alternative repository instead of official / curated repository.
  - This feature is not stable yet. Using this feature will warn you about it and use at your own risk. `#362` `#365`
- `vrc-get-litedb` crate which is NativeAOT based LiteDB wrapper for vrc-get `#320`
- `vrc-get vcc` commands which is a command for vrc-get as a VCC project
  - This feature is disabled by default. pass `--features experimental-vcc` to cargo to enable this feature. `#384` 
  - `vrc-get vcc project list` to list projects `#369`
  - `vrc-get vcc project add <path>` to add a project to project list `#369`
  - `vrc-get vcc project remove <path>` to remove a project from project list `#369`
  - `vrc-get vcc unity list` to list unity in vcc's unity list `#382`
  - `vrc-get vcc unity add <path>` to add a unity to vcc's unity list `#382`
  - `vrc-get vcc unity remove <path>` to remove a unity from vcc's unity list `#382`
  - In addition, `vrc-get migrate unity 2022` command will get unity from the vcc's unity list `#382`

### Changed

### Deprecated

### Removed

### Fixed
- Warnings about index map `#350`
- `vrc-get repo remove` not working `#361`
- `writing local repo cache 'Repos/vrc-curated.json'` error `#365`

### Security

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

### Deprecated

### Removed

### Fixed
- Some process are not in parallel [`#278`](https://github.com/anatawa12/vrc-get/pull/278)
- Resolving prerelease packages from unlocked packages fails [`#284`](https://github.com/anatawa12/vrc-get/pull/284)

### Security

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

### Security

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

[0.1.0]: https://github.com/anatawa12/vrc-get/releases/tag/v0.1.0
