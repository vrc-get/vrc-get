use clap::Parser;

mod add;

#[derive(Parser)]
pub enum Command {
    #[command(subcommand)]
    Add(add::Command),
}

impl Command {
    pub async fn run(self) {
        match self {
            Command::Add(cmd) => cmd.run().await,
        }
    }
}
