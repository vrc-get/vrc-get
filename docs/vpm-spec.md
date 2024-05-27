# The VRChat Package Manager and vrc-get extensions.

## Abstract

The VRChat Package Manager (VPM) is a package manager for UPM Packages developed by VRChat to distribute VRChat SDK, 
which is proprietary software, based on Hypertext Transfer Protocol (HTTP).
This document describes the overall architecture of VPM, defines how the VPM Client should work, and how the package repository should provide packages.
In addition, this document describes extensions of the vpm implemented by vrc-get.
Please note that this document is not provided by VRChat, is provided by vrc-get developers, a third party VPM Client developers.

## 1. Introduction
### 1.1. Purpose
The VRChat Package Manager is a package manager developed by VRChat and widely used by VRChat community.
However, the VRChat Creator Companion (VCC), the official VPM implementation, has several bugs that make it hard to know the VPM Client should work.
Therefore, vrc-get developers assume the best VPM behavior based on the behavior of VCC, and [[VCC Docs]].
This document describes the vrc-get developers' understanding of the VPM and vrc-get extensions.

### 1.2. Core Concepts
The VPM provides a simple way to distribute UPM packages.

The VPM Repository provides a list of packages and their metadata, and the link to the package archive file, in JSON document hosted on the HTTP server.
The VPM Repository can authorize VPM Clients by using Header-based authentication, including downloading the package archive file.

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
### 3.1. VPM, VPM Client, VPM Repository, VPM Profile Folder, and VPM Package
The term "VPM" refers to the VRChat Package Manager, which is the package manager system.
The VPM consists of the VPM Client and the VPM Repository.

The VPM Client is the software that manages the VPM Packages in the Unity project.
The VPM Client fetches the package information from the VPM Repository and installs the package.
The VPM Client acts as a User Agent in the HTTP request to the VPM Repository.

The VPM Clients usually have feature to manage VPM Repository installed for the VPM Manifest folder.
The VPM Client **MAY** have feature to add VPM Repository from Web Browsers by using the `vcc:` URL scheme.

The VPM Repository is the server that provides the package information and the package archive file.
The VPM Repository provides the package information in [[JSON]] format and the package archive file in Unity Package format.
One single VPM Repository can be consists of multiple servers, such as the package information server and the package archive server.
The VPM Repository acts as an HTTP server to provide the package information and the package archive file.

As described above, the VPM Client and VPM Repository communicate with each other by using the HTTP protocol [[HTTP]].

VPM Profile Folder is the folder that contains the configuration files and the cache files of the VPM Client.
The configuration file for VPM Client should be shared among the VPM Clients because it contains the user-wide installed repositories list.
However, the VPM Client **MAY** have different configuration files for some purposes, such as beta testing.
Actually, the recent VCC Beta uses their own folder as the VPM Profile Folder.

The VPM Package is the package distributed by the VPM.
The VPM Package will be recognized as a UPM Package by Unity Editor.
Therefore, the VPM Package is a special type of UPM Package.

### 3.2. The UPM, UPM Package
The UPM is the Unity Package Manager, which is the package manager developed by Unity Technologies and built into Unity Editor.
The UPM provides the package management system for Unity Editor.

The UPM Package is the package recognized by the UPM and Unity Editor.
All VPM Packages are recognized as Embedded UPM Packages by Unity Editor.

### 3.3. Package Manifest, VPM Manifest
The Package Manifest is the JSON document that describes the package information.
The Package Manifest is located as `package.json` in the UPM Package and provides the package information,
such as the package name, version, dependencies, and the package archive file name.

The VPM Manifest is the JSON document that describes the installed package information.
The VPM Manifest is located as `Packages/vpm-manifest.json` in the Unity project and provides the installed package information,
The VPM Manifest has two main sections, `dependencies` and `locked`.
The `dependencies` section provides the requested package information.
The `locked` section provides the all installed package information, including the dependencies of the requested packages.

### 3.4. Requested Package, Dependency Package, Locked Package, and Unlocked Package
The requested package means the VPM Packages that user requested to install.
The user can request to install the package by using the VPM Client.

The dependency package means the packages that are required to install the requested package.
The VPM Client automatically installs the dependency packages when installing the requested package.

The locked package means the Embedded packages that are controlled by the VPM Client.
The VPM Client manages the locked packages based on the VPM Manifest.

The unlocked package means the Embedded packages that are not controlled by the VPM Client.
The VPM Client does not manage the unlocked packages, but may read the package information from the package manifest.

## References
### \[VCC Docs]
VRChat provides documentation for VCC at https://vcc.docs.vrchat.com/.
The document is also published on GitHub at https://github.com/vrchat-community/creator-companion.
This spec is based on [2f09cfe](https://github.com/vrchat-community/creator-companion/tree/2f09cfef3734b34e6e2cf4d8107c955c4f123322).

### \[HTTP]
Fielding, R., Ed., Nottingham, M., Ed., and J. Reschke, Ed., "HTTP Semantics", STD 97, RFC 9110, DOI 10.17487/RFC9110, June 2022, <<https://www.rfc-editor.org/info/rfc9110>>.

### \[JSON]
Bray, T., Ed., "The JavaScript Object Notation (JSON) Data Interchange Format", STD 90, RFC 8259, DOI 10.17487/RFC8259, December 2017, <<https://www.rfc-editor.org/info/rfc8259>>.

### \[RFC2119]
Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, DOI 10.17487/RFC2119, March 1997, <<http://www.rfc-editor.org/info/rfc2119>>.

### \[RFC5234]
Crocker, D., Ed. and P. Overell, "Augmented BNF for Syntax Specifications: ABNF", STD 68, RFC 5234, DOI 10.17487/RFC5234, January 2008, <<https://www.rfc-editor.org/info/rfc5234>>.

### \[RFC8174]
Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, DOI 10.17487/RFC8174, May 2017, <<https://www.rfc-editor.org/info/rfc8174>>.

[VCC Docs]: #vcc-docs
[HTTP]: #http
[RFC5234]: #rfc5234
[RFC2119]: #rfc2119
[RFC8174]: #rfc8174
