use anyhow::*;

mod alcom_updater_json;
mod build_alcom;
mod bundle_alcom;
mod check_static_link;
mod sign_alcom;
mod utils;

trait Command {
    fn run(self) -> Result<i32>;
}

macro_rules! commands_def {
    (
        $(
        $(#[$attr:meta])*
        $name: ident = $module: ident;
        )*
    ) => {
        #[derive(clap::Parser)]
        enum Commands {
            $($(#[$attr])* $name($module::Command),)*
        }

        impl Command for Commands {
            fn run(self) -> Result<i32> {
                match self {
                    $(Commands::$name(cmd) => Command::run(cmd),)*
                }
            }
        }
    };
}

commands_def! {
    CheckStaticLink = check_static_link;
    AlcomUpdaterJson = alcom_updater_json;
    BuildAlcom = build_alcom;
    BundleAlcom = bundle_alcom;
    SignAlcom = sign_alcom;
}

fn main() -> Result<()> {
    let command: Commands = clap::Parser::parse();
    std::process::exit(command.run()?);
}
