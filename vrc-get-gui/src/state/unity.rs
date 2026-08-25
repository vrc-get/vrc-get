use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const UNITY_OPENING_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub struct UnityProjectState {
    opening: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    runtime_cache: Arc<Mutex<crate::os::UnityRuntimeCache>>,
}

impl UnityProjectState {
    pub fn new() -> Self {
        Self {
            opening: Arc::new(Mutex::new(HashMap::new())),
            runtime_cache: Arc::new(Mutex::new(crate::os::UnityRuntimeCache::new())),
        }
    }

    pub fn try_mark_opening(&self, project_path: impl Into<PathBuf>) -> bool {
        self.try_mark_opening_at(project_path.into(), Instant::now())
    }

    pub fn is_opening(&self, project_path: &Path) -> bool {
        self.is_opening_at(project_path, Instant::now())
    }

    pub fn clear_opening(&self, project_path: &Path) {
        self.opening.lock().unwrap().remove(project_path);
    }

    pub fn is_editor_ready(&self, project_path: &Path) -> bool {
        self.runtime_cache
            .lock()
            .unwrap()
            .is_editor_ready(project_path)
    }

    pub(crate) fn bring_unity_to_front(
        &self,
        project_path: &Path,
    ) -> std::io::Result<crate::os::BringUnityToFrontResult> {
        self.runtime_cache
            .lock()
            .unwrap()
            .bring_unity_to_front(project_path)
    }

    fn try_mark_opening_at(&self, project_path: PathBuf, now: Instant) -> bool {
        let mut opening = self.opening.lock().unwrap();
        remove_expired(&mut opening, now);

        if opening.contains_key(&project_path) {
            return false;
        }

        opening.insert(project_path, now);
        drop(opening);
        self.invalidate_runtime_cache();
        true
    }

    fn is_opening_at(&self, project_path: &Path, now: Instant) -> bool {
        let mut opening = self.opening.lock().unwrap();
        remove_expired(&mut opening, now);
        opening.contains_key(project_path)
    }

    fn invalidate_runtime_cache(&self) {
        self.runtime_cache.lock().unwrap().invalidate();
    }
}

fn remove_expired(opening: &mut HashMap<PathBuf, Instant>, now: Instant) {
    opening.retain(|_, started_at| now.duration_since(*started_at) < UNITY_OPENING_TIMEOUT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_marks_a_project_opening_once() {
        let state = UnityProjectState::new();
        let project = PathBuf::from("project");

        assert!(state.try_mark_opening(project.clone()));
        assert!(!state.try_mark_opening(project));
    }

    #[test]
    fn clearing_allows_a_project_to_be_marked_again() {
        let state = UnityProjectState::new();
        let project = PathBuf::from("project");

        assert!(state.try_mark_opening(project.clone()));
        state.clear_opening(&project);

        assert!(state.try_mark_opening(project));
    }

    #[test]
    fn opening_state_expires() {
        let state = UnityProjectState::new();
        let project = PathBuf::from("project");
        let started_at = Instant::now();

        assert!(state.try_mark_opening_at(project.clone(), started_at));
        assert!(state.is_opening_at(
            &project,
            started_at + UNITY_OPENING_TIMEOUT - Duration::from_millis(1)
        ));
        assert!(!state.is_opening_at(&project, started_at + UNITY_OPENING_TIMEOUT));
    }
}
