# Contributing

This project is based on

- TypeScript
- Rust
- C# .NET

By that mean, some dependencies are required to work with.
Each of them is defined in the project folder.

In addition, each project has its own contribution guidelines.
Please refer `CONTRIBUTING.md` file in the project folder.

## Projects

- [vrc-get CLI](vrc-get/README.md)
- [vrc-get LiteDB](vrc-get-litedb/README.md)
- [vrc-get GUI](vrc-get-gui/README.md)
- [vrc-get VPM](vrc-get-vpm/README.md)

## Configuration requirements

You can work on any OS system but this repository uses

- Git Submodules
- Symbolic Links

For Windows machines, you need to setup so your current user can create symbolic links. Refer to this documentation page <https://github.com/git-for-windows/git/wiki/Symbolic-Links>

To setup your project, use the following commands.

```bash
git clone --recurse-submodules https://github.com/vrc-get/vrc-get
```
