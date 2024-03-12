use std::ffi::OsStr;
use tokio::process::Command;

pub(crate) async fn start_command(_: &OsStr, path: &OsStr, args: &[&OsStr]) -> std::io::Result<()> {
    Command::new(path).args(args).spawn()?;
    Ok(())
}
