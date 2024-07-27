use crate::environment::vpm_settings::VpmSettings;
use crate::environment::vrc_get_settings::VrcGetSettings;
use crate::io;
use crate::io::EnvironmentIo;
use futures::future::try_join;

#[derive(Debug)]
pub struct Settings {
    /// parsed settings
    pub(super) vpm: VpmSettings,
    pub(super) vrc_get: VrcGetSettings,
}

impl Settings {
    pub async fn load(io: &impl EnvironmentIo) -> io::Result<Self> {
        let settings = VpmSettings::load(io).await?;
        let vrc_get_settings = VrcGetSettings::load(io).await?;

        Ok(Self {
            vpm: settings,
            vrc_get: vrc_get_settings,
        })
    }

    pub async fn save(&mut self, io: &impl EnvironmentIo) -> io::Result<()> {
        try_join(self.vpm.save(io), self.vrc_get.save(io))
            .await
            .map(|_| ())?;

        Ok(())
    }
}
