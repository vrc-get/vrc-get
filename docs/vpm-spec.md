# The VRChat Package Manager and vrc-get extensions.

## Abstract

The VRChat Package Manager (VPM) is a package manager architecture for UPM Packages
developed by VRChat to distribute VRChat SDK based on Hypertext Transfer Protocol (HTTP).
This document describes the overall architecture of VPM, 
how the VPM Client should work, and how the package repository should provide packages 
based on vrc-get developers' research.
In addition, this document describes extensions of the VPM implemented by vrc-get.
Please note that this document is not provided by VRChat, is provided by vrc-get developers, a third party VPM Client developers.

## 1. Introduction
### 1.1. Purpose
The VRChat Package Manager is a package manager developed by VRChat and widely used by the VRChat community.
However, the VRChat Creator Companion (VCC), the official VPM implementation, has several bugs that make it hard to know the VPM Client should work.
Therefore, vrc-get developers assume the best VPM behavior based on the behavior of VCC, and [[VCC Docs]].
This document describes the vrc-get developers' understanding of the VPM and vrc-get extensions
for future VPM Client developers and VPM Repository developers.

### 1.2. Core Concepts
The VPM provides a simple way to distribute UPM packages.

The VPM Repository provides a list of packages and their metadata, and the link to the package archive file, in JSON document hosted on the HTTP server.
The VPM Repository can authorize VPM Clients by using Header-based authentication, including downloading the package archive file.

The VPM Client is the software that manages the VPM Packages in the Unity project.
The VPM Client fetches the package information from Package Repositories and collects packages to install based on requested packages, version constraints, and dependencies.
The VPM Client then downloads the package archives from the URL provided by the Repository and extracts the package archive into `Packages` folder in the Unity project.
The VPM Client also writes the managing package information to `Packages/vpm-manifest.json` to manage the installed packages.
The VPM Client can re-install the package by using the information.
This information is usually used to install packages after cloning the project with Version Control System.
VPM Clients can share the user-wide installed repositories list and local package cache for better user experience.

## 2. Conformance
### 2.1. Syntax Notation
This specification uses the Augmented Backus-Naur Form (ABNF) notation of [[RFC5234]].

The following core rules are included by reference, as defined in Appendix B.1 of [[RFC5234]]: 
ALPHA (letters), CR (carriage return), CRLF (CR LF), CTL (controls), DIGIT (decimal 0-9), DQUOTE (double quote), 
HEXDIG (hexadecimal 0-9/A-F/a-f), HTAB (horizontal tab), LF (line feed), OCTET (any 8-bit sequence of data), SP (space), 
and VCHAR (any visible US-ASCII character).

### 2.2. Requirements Notation
The key words "**MUST**", "**MUST NOT**", "**REQUIRED**", "**SHALL**", "**SHALL NOT**", "**SHOULD**", 
"**SHOULD NOT**", "**RECOMMENDED**", "**NOT RECOMMENDED**", "**MAY**", and "**OPTIONAL**" 
in this document are to be interpreted as described in BCP 14 [[RFC2119]] [[RFC8174]] when, 
and only when, they appear in all capitals, as shown here.

## 3. Terminology and Core Semantics
### 3.1. The UPM, UPM Package
The UPM is the Unity Package Manager,
which is the package manager developed by Unity Technologies and built into Unity Editor.
The UPM provides the package management system for Unity Editor.

The UPM Package is the package recognized by the UPM and Unity Editor.
There are several ways to install UPM Packages to a Unity Project,
but for VPM, the embedded UPM Package is the most important.
The embedded UPM Package is the package that is placed in the `Packages` folder in the Unity project.

All VPM Packages are recognized as embedded UPM Packages by Unity Editor.

### 3.2. VPM, VPM Client, VPM Repository, VPM Profile Folder, and VPM Package
The term "VPM" refers to the VRChat Package Manager, which is the package manager architecture.
The VPM consists of the VPM Client and the VPM Repository.

The VPM Client is the software that manages the VPM Packages in the Unity project.
It fetches the package information from the VPM Repository and installs the package.

The VPM Clients usually have feature to manage VPM Repository installed for the VPM Manifest folder.
The VPM Client **MAY** have feature to add VPM Repository from Web Browsers by using the `vcc:` URL scheme.

The VPM Repository is a [[JSON]] file on a HTTP(S) Server that provides package manifest and URL to the package archive.

As described above, the VPM Client and VPM Repository communicate with each other via HTTP [[HTTP]].

VPM Profile Folder is the folder that contains the configuration files and the cache files of the VPM Client.
The configuration file for VPM Client should be shared among the VPM Clients because it contains the user-wide installed repositories list.
However, the VPM Client **MAY** have different configuration files for some purposes, such as beta testing.
Actually, the recent VCC Beta uses their own folder as the VPM Profile Folder.

The VPM Package is the package distributed by the VPM.
The VPM Package will be recognized as a UPM Package by Unity Editor.
Therefore, the VPM Package is a special type of UPM Package.

### 3.3. UPM Package Manifest, VPM Package Manifest, VPM Project Manifest
The UPM Package Manifest is the JSON document that describes the package information.
The UPM Package Manifest is located as `package.json` in the UPM Package and provides the package information,
such as the package name, version, UPM dependencies.

The VPM Package Manifest is a UPM Package Manifest that may have information for VPM.
In addition to the UPM Package Manifest,
the VPM Package Manifest provides the vpm dependencies and other data used by VPM.

The VPM Project Manifest is the JSON document that describes the installed package information.
The VPM Project Manifest is located as `Packages/vpm-manifest.json` in the Unity project and provides the installed package information,
The VPM Project Manifest has two main sections, `dependencies` and `locked`.
The `dependencies` section provides the requested package information.
The `locked` section provides the all installed package information, including the dependencies of the requested packages.

### 3.4. Requested Package, Dependency Package, Locked Package, and Unlocked Package
Requested packages are VPM Packages that the user requested to install.
The VPM client will tries to install those packages.

Dependency packages are VPM packages that are required to install the requested packages.
VPM Client automatically installs dependency packages when installing the requested package.

Locked packages are embedded UPM packages that are controlled by the VPM Client.
VPM Client manages locked packages based on the VPM Manifest.
All requested packages and Dependency Packages are locked packages.

Unlocked packages are the embedded UPM packages that are not controlled by the VPM Client.
VPM Client does not modify unlocked packages.
However, may read the package information from the UPM Package Manifest.

## 4. VPM Package and Manifest
This section describes the overall structure of VPM Package and the structure of the VPM Package Manifest.

### 4.1. Overall Structure of VPM Package
The VPM Package is a directory that contains the VPM Package Manifest file
(named `package.json`) and the entries of the package.

VPM Package **MUST** include file named `package.json` in the root directory of the package.
VPM Package consists of files and directories,
and they will be recognized as contents of the UPM Package by Unity Editor.

VPM Package **SHOULD NOT** contain the file-system entries other than files and directories such as symbolic links and device files.

All entries of the VPM Package **MUST** name with Unicode [[UNICODE]] characters
and **MUST NOT** include any characters that are not allowed in the file system.
All entries of the VPM Package **SHOULD NOT** use characters other than ASCII Alphabets, 
Digits, Hyphen (`-`), Underscore (`_`), Space (` `), Dot (`.`), and Parentheses (`(`, `)`).
Other characters can cause problems in some environments.

> [!NOTE]
> 
> The rules about the file name are not provided in the [[VCC Docs]]
>, and this is completely based on the vrc-get developers' research and opinion.
> 
> The official VCC uses .NET, so they cannot handle file names with non-UNICODE characters.
> Therefore, we wrote the rule that entry name MUST name with Unicode characters.
> 
> For non-UTF-8 environments, Unity is unstable with non-ASCII characters.
> Therefore, we wrote the rule that entry name SHOULD NOT use characters other than ASCII characters.
> 
> For some OSes like Windows, some characters are not allowed in the file name.
> Therefore, we wrote the rule that entry name SHOULD NOT use characters other than specified characters.

### 4.2. Structure of VPM Package Manifest
VPM Package Manifest is a [[JSON]] document that describes the package information.
The document **MUST** be named `package.json` and located in the root directory of the VPM Package.
The document **MUST** be encoded in UTF-8 [[UNICODE]].
VPM Package Manifest is a subset of the UPM Package Manifest.
Some fields are required, and some fields are optional.
Additional fields are allowed, so the VPM Client **SHOULD NOT** cause error with them.
Some VPM Clients **MAY** use the additional fields for their purposes.

#### 4.2.1. Basic Fields
The VPM Package Manifest **MUST** include the following two fields.

##### 4.2.1.1. `name` (string)
The `name` field shows the globally unique ID of the package.
The type of this field **MUST** be a string.

The package ID is used to identify single package, so this **MUST** be unique among all packages in the VPM Repository
and **SHOULD** be unique among all packages in the VPM ecosystem.
VPM client will treat packages with same ID as same package.

The Package ID **SHOULD** be prefixed with reverse domain name of the package author to be unique.

The package ID **SHOULD** be in lowercase and **MUST NOT** contain any characters other than ASCII Alphabets, Digits, Hyphen (`-`), and Dot (`.`).
The VPM Client **MAY** allow the package name that does not follow the rule.

##### 4.2.2.2. `version` (string)
The `version` field shows the version of the package.
The type of this field **MUST** be a string.
The value of this field **MUST** follow the Semantic Versioning 2.0.0 [[SEMVER]].
Because the UPM doesn't support that, the `version` **MUST NOT** include the build metadata.
The VPM Client **MAY** support a loose version like `"1.0"`, `"  v1.0.0 "` or `"v1.0.0"`,
however, some VPM Clients **MAY** ignore the packages with a loose version.

> [!NOTE]
> 
> The VCC currently does support the loose versions.
> This came from the underlying library of the VCC, `semver.net` by Adam Reeve (https://github.com/adamreeve/semver.net)
> In the future version of `semver.net`, accepting a loose version is going to be removed 
> so that the VCC may remove support the loose versions in the future.
>
> On the other hand, the vrc-get does not support the loose versions.
> This is originally came from the `semver` crate by David Tolnay.

#### 4.2.3. Package Resolution Fields
The VPM Package Manifest **MAY** include the following fields.
Those fields are used to determine the package is compatible with the Project and find dependency packages.

##### 4.2.3.1. `vpmDependencies` (object)
The `vpmDependencies` field shows the dependencies of the package.
The type of this field **MUST** be an object. 

The key of this field **MUST** be the package name,
and the value **MUST** be a string that describes the version constraint.
The format of the version constraint will be described in [!TODO:Section Version Constraint in Package Resolving].

The VPM Client will resolve the dependencies of the package based on this field.
More about the resolving process will be described in [!TODO:Section Package Resolving].

##### 4.2.3.2. `unity` (string)
The `unity` field shows the minimum Unity version that the package is compatible with.
The type of the this field **MUST** be a string.
The value of this field **MUST** follow the Unity Version number described in [!TODO:Section Unity Version in Package Resolving].

The VPM Client will check the compatibility of the package with the Unity Project based on the `unity` field.
If the package is not compatible with the Unity Project,
the VPM Client **MUST** reject the installation of the package or show a warning to the user. 

##### 4.2.3.3. `legacyPackages` (array)
The `legacyPackages` field shows the list of legacy packages that the package name is replacing.
The type of this field **MUST** be an array of strings.

The UdonSharp and ClientShim was provided as a separate package in the past
and since VRCSDK 3.4.0, they are provided as a part of the VRCSDK.
However, at that time, many packages are depending on the UdonSharp and ClientShim.
Therefore, VRChat introduced the `legacyPackages` field to provide the compatibility with the old packages.

The VPM Client **MUST** guarantee
that packages specified in the `legacyPackages` field are not installed when the package is installed.

#### 4.2.4. Installation Data Fields
The VPM Package Manifest **MAY** include the following fields that are used to install the package.

##### 4.2.4.1. `url` (string)
The `url` field shows the URL of the package archive file.
The type of this field **MUST** be a string.
The value of this field **MUST** be a valid URL that points to the package archive file.
The VPM Package Manifest in the Remote Package Repository **MUST** include the `url` field with the valid URL.

The VPM Client will download the package archive file from the URL specified with this field.
If the `url` field is not included in the VPM Package Manifest,
the VPM Client **MUST** reject the installation of the package.

The VPM Repository **MUST NOT** chnange the contents of the URL this field points to. tjis is compared bitwicely.

> [!NOTE]
> 
> Unfortunately, some known popular repository did change the contents of repository so it might be better to assume packages might change.

##### 4.2.4.2. `zipSHA256` (string)
The `zipSHA256` field shows the SHA-256 hash of the package archive file.
The type of the `zipSHA256` field **MUST** be a string.
The `zipSHA256` field **MUST** be a valid SHA-256 hash of the package archive file.

The VPM Client **MUST** use this hash to verify the integrity of the cached package archive file.
and **MAY** use this hash to verify the integrity of the downloaded package archive file.

The VPM Client **MAY** reject the installation of the package
if the downloaded package does not match the SHA-256 hash specified in the `zipSHA256` field.

##### 4.2.4.3. `headers` (object)
The `headers` field shows the additional HTTP headers that is for fetching the package archive file.
The type of the `headers` field **MUST** be an object, and the keys and values **MUST** be strings.

The VPM Client **MUST** add the headers specified in the `headers` field to fetch the package archive file.

#### 4.2.5. Human-readable Fields
The VPM Package Manifest **MAY** include the following fields that are used to provide Human-readable information of the package.
Those fields **MUST NOT** affect the behavior of the VPM Client except for errors on type mismatch.

##### 4.2.5.1. `displayName` (string)
The `displayName` field shows the human-readable name of the package.
The type of the `displayName` field **MUST** be a string.
The `displayName` field should show the human-readable name of the package.
The name is usually in English but may be in other languages.

##### 4.2.5.2. `description` (string)
The `description` field shows the human-readable description of the package.
The type of the `description` field **MUST** be a string.
The `description` field should show the human-readable description of the package.
The name is usually in English but may be in other languages.

##### 4.2.5.3. `changelogUrl` (string)
The `changelogUrl` field shows the URL of the changelog of the package.
The type of the `changelogUrl` field **MUST** be a string.
The `changelogUrl` field **SHOULD** be a valid URL that points to the changelog of the package.
The VPM Client **MAY** suggest users to open the URL with the Web Browser when the package is updated.

#### 4.2.A. `vrc-get` extension (object)
The VPM Package Manifest **MAY** include the `vrc-get` object that is used to provide the additional information for the vrc-get.
The `vrc-get` object **MUST** be an object.
The VPM Client **MAY** ignore the `vrc-get` object.
The `vrc-get` object **MAY** include the following fields.

##### 4.2.A.1. `vrc-get.yanked` (string or boolean)
The `yanked` field shows the yanked status of the package.
The type of the this field **MUST** be a non-empty string or boolean.
If this field is a string, the package is yanked and the string **MUST** be the reason of the yanked.
If this field is a boolean, the package is yanked if the value is `true` and not yanked if the value is `false`.

The VPM Clients that recognizes this field **SHOULD** show the warning
if the package is yanked and the package is already installed
and **MUST** deny installing yanked packages except for resolving the packages.

##### 4.2.A.2. `vrc-get.aliases` (array)
The `vrc-get.aliases` field shows the list of the alternative package display names.
The type of this field **MUST** be an array of strings.
The VPM Client **MAY** use this field to better package search
but this field **MUST NOT** affect the behavior of the VPM Client.

## 5. VPM Repository
The VPM Repository is a JSON file on a HTTP server that provides package manifest and the URL to the VPM Package Archive.
The user provides the URL and Headers information to the VPM Client to access the VPM Repository.

This section describes not only about contents of VPM Repository JSON file, but also about archive file VPM Repository points to.

### 5.1.  VPM Repository JSON
VPM Repository **MUST** be a JSON file encoded in UTF-8 [[UNICODE]] and **MUST** be an object that contains the following fields.

#### 5.1.1. `packages` (object)
The `packages` field shows the list of packages in the VPM Repository.
The type of this field **MUST** be an object.
The key of the value of this field **MUST** be the package ID
and the value **MUST** be an object that contains the following fields.

##### 5.1.1.1. `packages.<id>.versions` (object)
The `packages.<id>.versions` field shows the list of versions of the package.
The type of this field **MUST** be an object.
The key of the value of this field **MUST** be the version of the package,
which is a valid version name described in Semantic Versioning 2.0.0 [[SEMVER]].

The value **MUST** be an object that contains the VPM Package Manifest,
which is described in [Section 4.2](#42-structure-of-vpm-package-manifest).

The VPM Package Manifest **MUST** have same package ID as the key of `packages` object,
and same version as the key of `packages.<id>.versions` object.
The VPM Package Manifest **MUST** have the `url` field that points to the package archive file.

#### 5.1.2. `url` (string)
The `url` field shows the URL of the VPM Repository JSON.
The type of this field **MUST** be a string.
The value of this field **MUST** be a valid URL that points to the VPM Repository JSON.

According to The [[VCC Docs]], It should point to the VPM Repository JSON.
However, for Some existing VPM Repositories,
the `url` field is used to point some related pages like booth page or the Author's Website.
Therefore, VPM Client **SHOULD NOT** use the `url` field to fetch the VPM Repository JSON.

#### 5.1.3. `id` (string)
The `id` field shows the unique identifier of the VPM Repository.
The type of this field **MUST** be a string.
VPM Repository **SHOULD** provide the `id` field to identify the VPM Repository.
If the `id` field is not provided, the VPM Client use the URL of the VPM Repository as the `id` field.

The `id` field **MUST** be unique among all VPM Repositories.
To the `id` field be unique among all repositories in the VPM ecosystem, 
the `id` **SHOULD** be prefixed with reverse domain name of the VPM Repository author.

#### 5.1.4. `name` (string)
The `name` field shows the human-readable name of the VPM Repository.
The type of the `name` field **MUST** be a string.
The `name` field should show the human-readable name of the VPM Repository.

#### 5.1.5. `author` (string)
The `author` field shows the human-readable author of the VPM Repository.
The type of the `author` field **MUST** be a string.
The `author` field should show the human-readable author of the VPM Repository.

> [!NOTE]
> 
> In the VRChat Official Repository at <https://packages.vrchat.com/official?download> and other popular repositories 
> the `author` field shows the name of the organization or the group that manages the repository.
> 
> On the other hand, in the Example Repository in the [[VCC Docs]],
> the `author` field shows the email address of the author.
>
> We don't know the reason for this difference, but We think the `author` field should show the human-readable name of the author.
> It's more user-friendly and easier to understand.

### 5.2. Package Archive File
The Package Archive File is a Zip archive file that contains the package contents.
The Package Manifest inside the Package Repository JSON **MUST** have the `url` field
that points to the Package Archive File.

The paths in the Package Archive File **MUST** be encoded in UTF-8 [[UNICODE]]. It's not necessary to set Language Encoding flag in zip.

The root directory of the Package Archive File is the root directory of the VPM Package.
In other words, `package.json` of the VPM Package will be located as `package.json` in the Package Archive File,
not as `package/package.json` or `root/package.json`.

The VPM Package **MUST** use `/` as path separator, **MUST NOT** include paths starting with `/` (absolute path) or `../` (path traversal), or contains `\\` (Windows path separator), `/../` (path traversal), `/./` (path identity in archive), or `:` (Windows drive letter which can cause absolute path).

The VPM Client **MUST** support compression method 0 (stored) and 8 (deflate). Package Archive Files **SHOULD NOT** use compression Methods other than 0 or 8.

> [!NOTE]
> 
> Notes for the VPM Package Developers
> 
> In the recent OSes, when we compress a directory, they create root directory that contains the directory.
> Therefore, please be careful when you compress your VPM Package.
>
> In addition, you should not use Windows Explorer to create zip files since it may use deflate64, which will cause compatibility problems.

> [!NOTE]
> Notes About Deflate64 Compression Method
> 
> For better compatibility, the VPM Archive File should not use the compression method other than Deflate.
> 
> Most implementation of Zip file only supports the Deflate compression method or store uncompressed bytes.
> For example, the standard library of go, python,
> and java only supports the Deflate compression method and uncompressed bytes.
> 
> However, the Microsoft Windows built-in Zip file compressor may use the Deflate64 compression method.
> The Deflare64 compression method is not supported by most of the Zip file implementations.
> As a result, some existing VPM Packages may use the Deflate64 compression method in their Package Archive File.
> Therefore, it might be better for VPM Clients to support the Deflate64 compression method.
> 
> Side Note: the Deflate64 compression method is not publicly documented.
> In the APPNOTE.TXT by pkware, deflate64 is described as "Deflate64(tm) is supported by the Deflate extractor.",
> but it requires larger window size and no literal to code mapping on document, so it's impossible to decompress with existing Deflate implementation.
> In addition, the Deflate64 is not documented in any public documents,
> so there are very limited deflate 64 implementations.
> Here's an incomplete list of known open-source implementations:
> - [.NET System.IO.Compression](https://github.com/dotnet/runtime/blob/2f08fcbfece0c09319f237a6aee6f74c4a9e14e8/src/libraries/System.IO.Compression/src/System/IO/Compression/DeflateManaged/) by Microsoft, inc. under the MIT License
> - [7zip](https://sourceforge.net/projects/sevenzip/files/7-Zip/) by Igor Pavlov under the GNU LGPL 2.1
> - [deflate64-rs](https://github.com/anatawa12/deflate64-rs) by anatawa12 based which is reimplementation of the .NET System.IO.Compression under the MIT License
> - [Info Zip](https://sourceforge.net/projects/infozip/) by Info-ZIP under the Info-ZIP License
> - [Apache Commons Compress](https://commons.apache.org/proper/commons-compress/) by Apache Software Foundation under the Apache License 2.0

## References
### \[VCC Docs]
VRChat provides documentation for VCC at https://vcc.docs.vrchat.com/.
The document is also published on GitHub at https://github.com/vrchat-community/creator-companion.
This spec is based on commit [2f09cfe](https://github.com/vrchat-community/creator-companion/tree/2f09cfef3734b34e6e2cf4d8107c955c4f123322).

### \[HTTP]
Fielding, R., Ed., Nottingham, M., Ed., and J. Reschke, Ed., "HTTP Semantics", STD 97, RFC 9110, DOI 10.17487/RFC9110, June 2022, <<https://www.rfc-editor.org/info/rfc9110>>.

### \[JSON]
Bray, T., Ed., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, DOI 10.17487/RFC8259, December 2017, <<https://www.rfc-editor.org/info/rfc8259>>.

### \[UNICODE]
Unicode Consortium, "The Unicode Standard", <<https://www.unicode.org/versions/latest/>>.

### \[SEMVER]
Preston-Werner, T., "Semantic Versioning 2.0.0", <<https://semver.org/spec/v2.0.0.html>>.

### \[RFC2119]
Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, DOI 10.17487/RFC2119, March 1997, <<http://www.rfc-editor.org/info/rfc2119>>.

### \[RFC5234]
Crocker, D., Ed. and P. Overell, "Augmented BNF for Syntax Specifications: ABNF", STD 68, RFC 5234, DOI 10.17487/RFC5234, January 2008, <<https://www.rfc-editor.org/info/rfc5234>>.

### \[RFC8174]
Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, DOI 10.17487/RFC8174, May 2017, <<https://www.rfc-editor.org/info/rfc8174>>.

[VCC Docs]: #vcc-docs
[HTTP]: #http
[JSON]: #json
[SEMVER]: #semver
[UNICODE]: #unicode
[RFC5234]: #rfc5234
[RFC2119]: #rfc2119
[RFC8174]: #rfc8174
