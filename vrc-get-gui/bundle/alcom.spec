Name:           alcom
Version:        1.1.6
Release:        1%{?dist}
Summary:        A short description of my custom application

%define commit gui-v%{version}

License:        MIT
URL:            https://vrc-get.anatawa12.com/alcom/
Source0:        https://github.com/vrc-get/vrc-get/archive/%{commit}.tar.gz

BuildRequires:  gcc
BuildRequires:  nodejs
BuildRequires:  npm
BuildRequires:  pkgconfig(gtk+-3.0)
BuildRequires:  pkgconfig(webkit2gtk-4.1)
BuildRequires:  pkgconfig(openssl)

# we download rust toolchain manually when building inside mock container
%if ! 0%{?install_rust:1}
BuildRequires:  cargo
%endif

# disable stripping symbols.
%global __os_install_post %{nil}
%global debug_package %{nil}

%description
ALCOM - Alternative Creator Companion
ALCOM is a fast and open-source alternative VCC (VRChat Creator Companion) written in rust and tauri.

%prep
%setup -q -n vrc-get-%{commit}

%if 0%{?install_rust:1}
    echo "=== Mock environment detected. Installing isolated Rust toolchain ==="

    export RUSTUP_HOME="$(pwd)/.rustup"
    export CARGO_HOME="$(pwd)/.cargo"

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path

    cat << EOF > ./load_rust_env.sh
    export RUSTUP_HOME="${RUSTUP_HOME}"
    export CARGO_HOME="${CARGO_HOME}"
    source "${CARGO_HOME}/env"
EOF
%endif

# marker: ci inserts version update here

%build
%{?install_rust: source ./load_rust_env.sh}
cargo xtask build-alcom --release

%install
%{?install_rust: source ./load_rust_env.sh}
rm -rf %{buildroot}
cargo xtask bundle-alcom --release --bundles buildroot --buildroot=%{buildroot}

%files
%license LICENSE
# %doc vrc-get-gui/README.md
 #%doc vrc-get-gui/CHANGELOG.md
%{_bindir}/alcom
%{_datadir}/applications/alcom.desktop
%{_datadir}/icons/hicolor/*/apps/alcom.png

%changelog
* Migrated to native rpm build pipeline with spec file
