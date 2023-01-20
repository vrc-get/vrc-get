use clap::Parser;

mod add;
mod resolve;

#[derive(Parser)]
pub enum Command {
    #[command(subcommand)]
    Add(add::Command),
    Resolve(resolve::Command),
}

impl Command {
    pub async fn run(self) {
        match self {
            Command::Add(cmd) => cmd.run().await,
            Command::Resolve(cmd) => cmd.run().await,
        }
    }
}
