use crate::bundle_alcom::BundleContext;
use crate::bundle_alcom::linux::*;
use crate::utils::tar::TarBuilderExt;
use crate::utils::{CountingIo, tar, target_arch};
use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::{fs, io};

fn deb_arch(triple: &str) -> Result<&str> {
    match target_arch(triple) {
        "aarch64" => Ok("arm64"),
        "x86_64" => Ok("amd64"),
        _ => {
            bail!(
                "unsupported architecture in target triple for deb: {}",
                triple
            )
        }
    }
}

pub fn create_deb(ctx: &BundleContext<'_>) -> Result<()> {
    let arch = deb_arch(ctx.target_tuple)?;
    let pkg_name = format!("alcom_{}-1_{arch}", ctx.version());

    let (estimated_size, data_tar_gz) = {
        let gz = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = tar::Builder::new(CountingIo::new(gz));

        tar.append_directory("usr").context("adding /usr")?;
        tar.append_directory("usr/bin").context("adding /usr/bin")?;

        let bin_name_lower = ctx.binary_name().to_ascii_lowercase();
        tar.append_data(
            tar::HeaderBuilder::new_gnu().with_mode(0o755).build(),
            format!("usr/bin/{bin_name_lower}"),
            fs::File::open(ctx.binary_path()).context("reading executable")?,
        )
        .context("adding /usr/bin/{bin_name_lower}")?;

        tar.append_directory("usr/share")
            .context("adding /usr/share")?;
        tar.append_directory("usr/share/applications")
            .context("adding /usr/share/applications")?;

        let bin_name_lower = ctx.binary_name().to_ascii_lowercase();
        tar.append_data(
            tar::HeaderBuilder::new_gnu().with_mode(0o755).build(),
            format!("usr/share/applications/{bin_name_lower}.desktop"),
            io::Cursor::new(
                render_desktop_file(ctx, &format!("/usr/bin/{bin_name_lower}"))?.as_bytes(),
            ),
        )
        .context("adding /usr/share/applications/{bin_name_lower}.desktop")?;

        tar.append_directory("usr/share/icons")
            .context("adding /usr/share/icons")?;
        tar.append_directory("usr/share/hicolor")
            .context("adding /usr/share/hicolor")?;

        for size in LINUX_ICON_RESOLUTIONS {
            tar.append_directory(format!("usr/share/hicolor/{size}"))
                .with_context(|| format!("adding /usr/share/hicolor/{size}"))?;
            tar.append_directory(format!("usr/share/hicolor/{size}/app"))
                .with_context(|| format!("adding /usr/share/hicolor/{size}/app"))?;

            tar.append_data(
                tar::HeaderBuilder::new_gnu().with_mode(0o644).build(),
                format!("usr/share/hicolor/{size}/app/{LINUX_ICON_NAME}.png"),
                fs::File::open(ctx.icon_path(size)).context("reading icon")?,
            )
            .with_context(|| {
                format!("adding /usr/share/hicolor/{size}/app/{LINUX_ICON_NAME}.png")
            })?;
        }

        let finished_gz_count = tar.into_inner()?;
        let estimated_size = finished_gz_count.count();
        let finished_gz = finished_gz_count.into_inner();
        let data_tar_gz = finished_gz.finish().context("finishing data.tar.gz")?;

        (estimated_size, data_tar_gz)
    };

    // Build control.tar.gz
    let control_tar_gz = {
        let mut control_tar_gz = Vec::new();
        let gz = GzEncoder::new(&mut control_tar_gz, Compression::best());
        let mut tar = tar::Builder::new(gz);

        let control = {
            let template_path = ctx.gui_dir.join("bundle/deb-control");
            fs::read_to_string(&template_path)
                .with_context(|| format!("reading {}", template_path.display()))?
                .replace("{{version}}", ctx.version())
                .replace("{{arch}}", arch)
                .replace("{{estimated_size}}", &estimated_size.to_string())
        };

        tar.append_data(
            tar::HeaderBuilder::new_gnu()
                .with_size(control.len() as u64)
                .with_mode(0o644)
                .build(),
            "control",
            io::Cursor::new(control.as_bytes()),
        )
        .context("appending control file")?;

        let gz = tar.into_inner().context("finishing control tar")?;
        gz.finish().context("finishing control gzip")?;

        control_tar_gz
    };

    // Assemble .deb as an ar archive.
    let deb_dir = ctx.bundle_dir.join("deb");
    fs::create_dir_all(&deb_dir)?;
    let deb_name = format!("{pkg_name}.deb");
    let deb_out = deb_dir.join(&deb_name);

    {
        let deb_file = fs::File::create(&deb_out)
            .with_context(|| format!("creating {}", deb_out.display()))?;
        let mut builder = ar::Builder::new(deb_file);

        // debian-binary
        let debian_binary = b"2.0\n";
        let mut header = ar::Header::new(b"debian-binary".to_vec(), debian_binary.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut debian_binary.as_slice())
            .context("appending debian-binary")?;

        // control.tar.gz
        let mut header = ar::Header::new(b"control.tar.gz".to_vec(), control_tar_gz.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut control_tar_gz.as_slice())
            .context("appending control.tar.gz")?;

        // data.tar.gz
        let mut header = ar::Header::new(b"data.tar.gz".to_vec(), data_tar_gz.len() as u64);
        header.set_mode(0o100644);
        builder
            .append(&header, &mut data_tar_gz.as_slice())
            .context("appending data.tar.gz")?;
    }

    println!("created: {}", deb_out.display());
    Ok(())
}
