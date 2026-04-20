pub use ::tar::*;
use std::fs::Metadata;
use std::io;
use std::path::Path;

pub struct HeaderBuilder {
    inner: Header,
}

#[allow(dead_code)]
impl HeaderBuilder {
    #[inline]
    pub fn new_gnu() -> Self {
        Self {
            inner: Header::new_gnu(),
        }
    }

    #[inline]
    pub fn build(&mut self) -> &mut Header {
        self.inner.set_cksum();
        &mut self.inner
    }

    #[inline]
    pub fn with_metadata(&mut self, metadata: &Metadata) -> &mut Self {
        self.inner.set_metadata(metadata);
        self
    }

    #[inline]
    pub fn with_metadata_in_mode(&mut self, metadata: &Metadata, mode: HeaderMode) -> &mut Self {
        self.inner.set_metadata_in_mode(metadata, mode);
        self
    }

    #[inline]
    pub fn with_size(&mut self, size: u64) -> &mut Self {
        self.inner.set_size(size);
        self
    }

    #[inline]
    pub fn with_path<P: AsRef<Path>>(&mut self, path: P) -> io::Result<&mut Self> {
        self.inner.set_path(path)?;
        Ok(self)
    }

    #[inline]
    pub fn with_link_name<P: AsRef<Path>>(&mut self, name: P) -> io::Result<&mut Self> {
        self.inner.set_link_name(name)?;
        Ok(self)
    }

    #[inline]
    pub fn with_link_name_literal<P: AsRef<[u8]>>(&mut self, name: P) -> io::Result<&mut Self> {
        self.inner.set_link_name_literal(name)?;
        Ok(self)
    }

    #[inline]
    pub fn with_mode(&mut self, mode: u32) -> &mut Self {
        self.inner.set_mode(mode);
        self
    }

    #[inline]
    pub fn with_uid(&mut self, uid: u64) -> &mut Self {
        self.inner.set_uid(uid);
        self
    }

    #[inline]
    pub fn with_gid(&mut self, gid: u64) -> &mut Self {
        self.inner.set_gid(gid);
        self
    }

    #[inline]
    pub fn with_mtime(&mut self, mtime: u64) -> &mut Self {
        self.inner.set_mtime(mtime);
        self
    }

    #[inline]
    pub fn with_username(&mut self, username: &str) -> io::Result<&mut Self> {
        self.inner.set_username(username)?;
        Ok(self)
    }

    #[inline]
    pub fn with_groupname(&mut self, groupname: &str) -> io::Result<&mut Self> {
        self.inner.set_groupname(groupname)?;
        Ok(self)
    }

    #[inline]
    pub fn with_device_major(&mut self, device_major: u32) -> io::Result<&mut Self> {
        self.inner.set_device_major(device_major)?;
        Ok(self)
    }

    #[inline]
    pub fn with_device_minor(&mut self, device_minor: u32) -> io::Result<&mut Self> {
        self.inner.set_device_minor(device_minor)?;
        Ok(self)
    }

    #[inline]
    pub fn with_entry_type(&mut self, entry_type: EntryType) -> &mut Self {
        self.inner.set_entry_type(entry_type);
        self
    }
}

pub trait TarBuilderExt {
    fn append_directory<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>;
}

impl<W: io::Write> TarBuilderExt for Builder<W> {
    fn append_directory<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.append(
            HeaderBuilder::new_gnu()
                .with_mode(0o755)
                .with_entry_type(EntryType::Directory)
                .with_path(path)?
                .build(),
            io::empty(),
        )
    }
}
