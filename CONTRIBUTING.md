# Contributing

First, thank you for considering contributing to this project.

This project consists of multiple projects.
Each project may have its own contribution guidelines, so please read the `CONTRIBUTING.md` file in the project folder.

- [vrc-get CLI](vrc-get/CONTRIBUTING.md) (not available yet)
- [vrc-get GUI](vrc-get-gui/CONTRIBUTING.md)
- [vrc-get VPM](vrc-get-vpm/CONTRIBUTING.md) (not available yet)

## Setup development environment

### System configuration requirements

This project is generally based on Rust, so you need to install Rust to work with this project.

Please refer to the [Rust installation guide](https://www.rust-lang.org/tools/install) to install Rust if you don't have it.

This project is constantly updated to the latest stable version of Rust,\
so it is recommended to install with rustup and update it regularly.

Not only Rust, some projects may require additional dependencies.

For ALCOM, you need to install Node.js >=20 supported and npm for building the frontend.

Please refer to the [Node.js installation guide](https://nodejs.org/en/download/) to install Node.js if you don't have it.

### Clonging requirements

To set up your project, use the following commands.

```bash
git clone https://github.com/vrc-get/vrc-get.git
# or
git clone --recurse-submodules https://github.com/vrc-get/vrc-get.git
```

You can work on any OS system, but this repository generally uses Symbolic Links.

For Windows machines, you may need to set up so your current user can create symbolic links.
Please refer to git-for-windows documentation page <https://github.com/git-for-windows/git/wiki/Symbolic-Links>
