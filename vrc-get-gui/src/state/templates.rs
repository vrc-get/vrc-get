use crate::templates::ProjectTemplateInfo;
use arc_swap::ArcSwapOption;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

type Data = Vec<ProjectTemplateInfo>;

struct TemplatesStateInner {
    pub version: u32,
    pub data: Data,
}

impl TemplatesStateInner {
    fn new_version() -> u32 {
        static VERSION: AtomicU32 = AtomicU32::new(0);
        VERSION.fetch_add(1, Ordering::AcqRel)
    }

    pub fn new(data: Data) -> Self {
        let version = Self::new_version();
        Self { version, data }
    }
}

pub struct TemplatesState {
    inner: ArcSwapOption<TemplatesStateInner>,
}

impl TemplatesState {
    pub fn new() -> Self {
        Self {
            inner: ArcSwapOption::new(None),
        }
    }

    pub fn save(&self, templates: Data) -> TemplatesStateRef<'_> {
        let arc = Arc::new(TemplatesStateInner::new(templates));
        self.inner.store(Some(arc.clone()));
        TemplatesStateRef {
            arc,
            _phantom_data: PhantomData,
        }
    }

    pub fn get(&self) -> Option<TemplatesStateRef<'_>> {
        let loaded = self.inner.load_full()?;
        Some(TemplatesStateRef {
            arc: loaded,
            _phantom_data: PhantomData,
        })
    }

    pub fn get_versioned(&self, version: u32) -> Option<TemplatesStateRef<'_>> {
        let loaded = self.inner.load_full()?;
        if loaded.version == version {
            Some(TemplatesStateRef {
                arc: loaded,
                _phantom_data: PhantomData,
            })
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn clear_cache(&self) {
        self.inner.store(None);
    }
}

pub struct TemplatesStateRef<'a> {
    arc: Arc<TemplatesStateInner>,
    _phantom_data: PhantomData<&'a ()>,
}

impl Deref for TemplatesStateRef<'_> {
    type Target = [ProjectTemplateInfo];

    fn deref(&self) -> &Self::Target {
        self.templates()
    }
}

impl TemplatesStateRef<'_> {
    pub fn templates(&self) -> &[ProjectTemplateInfo] {
        self.arc.data.deref()
    }

    pub fn version(&self) -> u32 {
        self.arc.version
    }
}
