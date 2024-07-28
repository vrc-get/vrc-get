macro_rules! with_environment {
    ($state: expr, |$environment: pat_param| $body: expr) => {{
        let mut state = $state.lock().await;
        let state = &mut *state;
        let $environment = state
            .environment
            .get_environment_mut(
                $crate::state::UpdateRepositoryMode::None,
                &state.io,
                &state.http,
            )
            .await?;
        $body
    }};
}
