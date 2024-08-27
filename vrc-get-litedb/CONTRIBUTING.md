# Contributing vrc-get-litedb

First, please read [CONTRIBUTING.md of the entire project](../CONTRIBUTING.md)!

## Setup development environment

### System configuration requirements

This project is based on .NET 8 NativeAOT and Rust, so you need to install .NET 8.0 SDK and Rust to work with this project.

Please refer to the [.NET installation guide](https://dotnet.microsoft.com/download) to install .NET SDK if you don't have it.
In addition, please refer to [Prerequisites for NativeAOT](https://learn.microsoft.com/ja-jp/dotnet/core/deploying/native-aot/?tabs=net8plus%2Cwindows#prerequisites) to install the required tools.

## Notes for developers

This crate is currently mono-repoed with the vrc-get project, however, other crates, vrc-get, vrc-get-vpm, and vrc-get-gui are not using this crate directly,
they're using the vrc-get-litedb crate published on crates.io.

Therefore, to test the change, you need to change version name and patch dependencies in the vrc-get project to use the local version of this crate like following.

```toml
[patch.crates-io]
vrc-get-litedb = { path = "vrc-get-litedb" }
```
