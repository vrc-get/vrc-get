use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Command {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
}

impl Command {
    pub(crate) async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");
        let mut unity = crate::vpm::UnityProject::find_unity_project(self.project)
            .await
            .expect("unity project not found");

        unity.resolve(&mut env).await.expect("resolve");

        unity.save().await.expect("saving manifest file");
    }
}
