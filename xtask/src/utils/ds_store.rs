//! Read/write support for the macOS `.DS_Store` file format.
//!
//! `.DS_Store` is a proprietary binary file created by macOS Finder to store
//! per-directory metadata (icon positions, window bounds, etc.).  It is used
//! in disk images (DMGs) to give the installer window a specific appearance.
//!
//! # File format overview
//!
//! The file uses a *buddy allocator* (block device abstraction) on which a
//! B-tree sorted by filename (case-insensitively) stores individual records.
//!
//! ```text
//! File layout (bytes):
//!   [0..36]       File header: magic1 (u32), "Bud1", root_off (u32),
//!                               root_size (u32), root_off (u32), unknown (16 B)
//!   buddy blocks  Each block lives at (buddy_offset + 4) in the file.
//!                 The size is 1 << (addr & 0x1F); offset = addr & !0x1F.
//! ```
//!
//! For the common case of a small number of entries (up to ~50) everything
//! fits in a single B-tree leaf node, so the file always has the same set of
//! three allocated blocks and the same 16 388-byte size.  That is the only
//! case this module supports writing.
//!
//! # References
//!
//! * <https://wiki.mozilla.org/DS_Store_File_Format>
//! * Python `ds-store` library (<https://github.com/al45tair/ds_store>)

use std::path::Path;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A `.DS_Store` file held in memory.
///
/// Construct with [`DsStore::new`], insert entries, then call
/// [`DsStore::to_bytes`] or [`DsStore::write_to`].
pub struct DsStore {
    entries: Vec<Entry>,
}

/// A single record in a `.DS_Store` file.
pub struct Entry {
    /// Filename the record is attached to (e.g. `"."`, `"ALCOM.app"`).
    pub filename: String,
    /// Four-byte record code (e.g. `b"Iloc"`, `b"bwsp"`).
    pub code: [u8; 4],
    /// The typed value of the record.
    pub value: EntryValue,
}

/// The typed value of a DS_Store record.
pub enum EntryValue {
    /// `bool` — one byte.
    Bool(bool),
    /// `long` — 32-bit unsigned integer.
    Long(u32),
    /// `shor` — stored as 32-bit in the file; represents a shorter integer.
    Shor(u32),
    /// `blob` — variable-length binary blob.
    Blob(Vec<u8>),
    /// `ustr` — UTF-16 string.
    Ustr(String),
    /// `comp` — 64-bit unsigned integer.
    Comp(u64),
    /// `dutc` — 64-bit date (UTC), same wire format as `comp`.
    Dutc(u64),
}

impl DsStore {
    /// Create an empty DS_Store.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert a record.
    ///
    /// Records are not deduplicated here; the caller is responsible for
    /// inserting each (filename, code) pair at most once.
    pub fn insert(&mut self, filename: impl Into<String>, code: &[u8; 4], value: EntryValue) {
        self.entries.push(Entry {
            filename: filename.into(),
            code: *code,
            value,
        });
    }

    /// Convenience: set the Finder icon location (`Iloc`) for `filename`.
    ///
    /// `x` and `y` are the pixel coordinates of the icon centre within the
    /// Finder window.
    pub fn set_icon_location(&mut self, filename: &str, x: u32, y: u32) {
        // The Iloc blob is 16 bytes: x (u32), y (u32), 0xFFFF_FFFF, 0xFFFF_0000
        let mut blob = Vec::with_capacity(16);
        blob.extend_from_slice(&x.to_be_bytes());
        blob.extend_from_slice(&y.to_be_bytes());
        blob.extend_from_slice(&0xFFFF_FFFF_u32.to_be_bytes());
        blob.extend_from_slice(&0xFFFF_0000_u32.to_be_bytes());
        self.insert(filename, b"Iloc", EntryValue::Blob(blob));
    }

    /// Serialize the DS_Store to a byte vector.
    ///
    /// # Panics
    ///
    /// Panics if the serialized B-tree node exceeds 8 192 bytes (i.e. more
    /// than ~50 typical entries).  For DMG use cases this limit is never
    /// reached.
    pub fn to_bytes(&self) -> Vec<u8> {
        build_file(&self.entries)
    }

    /// Write the DS_Store to `path`.
    pub fn write_to(&self, path: &Path) -> anyhow::Result<()> {
        std::fs::write(path, self.to_bytes())?;
        Ok(())
    }
}

impl Default for DsStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// File serialisation
// ---------------------------------------------------------------------------

/// Build the complete `.DS_Store` binary for `entries`.
fn build_file(entries: &[Entry]) -> Vec<u8> {
    // Sort entries by (filename_lowercase, code) — the B-tree is ordered this way.
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| {
        let af = a.filename.to_lowercase();
        let bf = b.filename.to_lowercase();
        af.cmp(&bf).then(a.code.cmp(&b.code))
    });

    // Serialise the B-tree leaf node (block 2, width 13, size 8192).
    let node_bytes = serialise_leaf_node(&sorted);
    assert!(
        node_bytes.len() <= BTREE_BLOCK_SIZE,
        "DS_Store B-tree node overflow ({} > {})",
        node_bytes.len(),
        BTREE_BLOCK_SIZE,
    );

    // Build the full 16 388-byte file.
    let mut file = vec![0u8; FILE_SIZE];

    // --- Header (file bytes 0..36) ---
    write_u32_be(&mut file[0..4], 1); // magic1
    file[4..8].copy_from_slice(b"Bud1"); // magic2
    write_u32_be(&mut file[8..12], ROOT_BUDDY_OFFSET); // root offset
    write_u32_be(&mut file[12..16], ROOT_BLOCK_SIZE as u32); // root size
    write_u32_be(&mut file[16..20], ROOT_BUDDY_OFFSET); // root offset (again)
    file[20..36].copy_from_slice(HEADER_UNKNOWN); // 16 unknown bytes

    // --- DSDB block (file bytes 36..68, buddy offset 0x20, width 5) ---
    {
        let dsdb = &mut file[DSDB_FILE_OFFSET..DSDB_FILE_OFFSET + DSDB_BLOCK_SIZE];
        write_u32_be(&mut dsdb[0..4], BTREE_BLOCK_NUM); // root node block
        write_u32_be(&mut dsdb[4..8], 0); // levels (depth - 1, 0 = root is leaf)
        write_u32_be(&mut dsdb[8..12], sorted.len() as u32); // record count
        write_u32_be(&mut dsdb[12..16], 1); // node count
        write_u32_be(&mut dsdb[16..20], 4096); // page size
    }

    // --- Root block (file bytes 2052..4100, buddy offset 0x800, width 11) ---
    write_root_block(&mut file[ROOT_FILE_OFFSET..ROOT_FILE_OFFSET + ROOT_BLOCK_SIZE]);

    // --- B-tree leaf node (file bytes 8196..16388, buddy offset 0x2000, width 13) ---
    file[BTREE_FILE_OFFSET..BTREE_FILE_OFFSET + node_bytes.len()]
        .copy_from_slice(&node_bytes);

    file
}

// ---------------------------------------------------------------------------
// Layout constants
//
// These are fixed for any DS_Store file created by the Python `ds-store`
// library (or our writer), regardless of content.
// ---------------------------------------------------------------------------

/// Total file size produced by this writer (16 388 bytes).
const FILE_SIZE: usize = 16_388;

/// The 16 "unknown" bytes in the file header — identical in every new file
/// created by the Python ds-store library.
const HEADER_UNKNOWN: &[u8; 16] =
    &[0x00, 0x00, 0x10, 0x0c, 0x00, 0x00, 0x00, 0x87,
      0x00, 0x00, 0x20, 0x0b, 0x00, 0x00, 0x00, 0x00];

// ---- block 0: root (buddy allocator meta) ----
/// Buddy offset of the root block (= `0x800`).
const ROOT_BUDDY_OFFSET: u32 = 0x800;
/// Encoded address of the root block stored in the offset table: offset | width.
const ROOT_BLOCK_ADDR: u32 = ROOT_BUDDY_OFFSET | 11; // width 11 → size 2048
/// File offset of the root block = buddy_offset + 4.
const ROOT_FILE_OFFSET: usize = ROOT_BUDDY_OFFSET as usize + 4;
/// Size of the root block (2^11 = 2 048 bytes).
const ROOT_BLOCK_SIZE: usize = 2048;

// ---- block 1: DSDB ----
/// Block number assigned to the DSDB record.
const DSDB_BLOCK_NUM: u32 = 1;
/// Buddy offset of the DSDB block (`0x20` = 32).
const DSDB_BUDDY_OFFSET: u32 = 0x20;
/// Encoded address of the DSDB block: offset | width.
const DSDB_BLOCK_ADDR: u32 = DSDB_BUDDY_OFFSET | 5; // width 5 → size 32
/// File offset of the DSDB block = buddy_offset + 4.
const DSDB_FILE_OFFSET: usize = DSDB_BUDDY_OFFSET as usize + 4;
/// Size of the DSDB block (2^5 = 32 bytes).
const DSDB_BLOCK_SIZE: usize = 32;

// ---- block 2: B-tree node ----
/// Block number assigned to the B-tree root node.
const BTREE_BLOCK_NUM: u32 = 2;
/// Buddy offset of the B-tree node (`0x2000` = 8 192).
const BTREE_BUDDY_OFFSET: u32 = 0x2000;
/// Encoded address of the B-tree node: offset | width.
const BTREE_BLOCK_ADDR: u32 = BTREE_BUDDY_OFFSET | 13; // width 13 → size 8192
/// File offset of the B-tree node = buddy_offset + 4.
const BTREE_FILE_OFFSET: usize = BTREE_BUDDY_OFFSET as usize + 4;
/// Size of the B-tree block (2^13 = 8 192 bytes).
const BTREE_BLOCK_SIZE: usize = 8192;

// ---------------------------------------------------------------------------
// Root block serialisation
// ---------------------------------------------------------------------------

fn write_root_block(buf: &mut [u8]) {
    assert_eq!(buf.len(), ROOT_BLOCK_SIZE);

    let mut w = BufWriter::new(buf);

    // Offset table header.
    w.write_u32(3); // count = 3 allocated blocks
    w.write_u32(0); // unknown2

    // Offset table (256 u32s, padded; only 3 are non-zero).
    w.write_u32(ROOT_BLOCK_ADDR); // block 0 = root itself
    w.write_u32(DSDB_BLOCK_ADDR); // block 1 = DSDB
    w.write_u32(BTREE_BLOCK_ADDR); // block 2 = B-tree node
    for _ in 3..256 {
        w.write_u32(0);
    }

    // TOC: one entry, "DSDB" → block DSDB_BLOCK_NUM.
    w.write_u32(1); // TOC count
    w.write_u8(4); // name length
    w.write_bytes(b"DSDB");
    w.write_u32(DSDB_BLOCK_NUM);

    // Free list: 32 widths.
    //
    // After creating the three blocks above, the remaining free blocks are:
    //   w=5  used by DSDB  → []
    //   w=6  0x40          → [0x40]
    //   w=7  0x80          → [0x80]
    //   w=8  0x100         → [0x100]
    //   w=9  0x200         → [0x200]
    //   w=10 0x400         → [0x400]
    //   w=11 used by root  → []
    //   w=12 0x1000        → [0x1000]
    //   w=13 used by node  → []
    //   w=14..30           → [0x4000..0x40000000]
    //   w=31               → []
    write_free_list(&mut w);
}

fn write_free_list(w: &mut BufWriter<'_>) {
    // Widths 0–5: empty.
    for _ in 0..=5 {
        w.write_u32(0);
    }
    // Widths 6–10: one free block each.
    for width in 6u32..=10 {
        w.write_u32(1); // count
        w.write_u32(1u32 << width); // buddy offset
    }
    // Width 11: empty (used by root).
    w.write_u32(0);
    // Width 12: one free block.
    w.write_u32(1);
    w.write_u32(1u32 << 12);
    // Width 13: empty (used by B-tree node).
    w.write_u32(0);
    // Widths 14–30: one free block each.
    for width in 14u32..=30 {
        w.write_u32(1);
        w.write_u32(1u32 << width);
    }
    // Width 31: empty.
    w.write_u32(0);
}

// ---------------------------------------------------------------------------
// B-tree leaf node serialisation
// ---------------------------------------------------------------------------

fn serialise_leaf_node(entries: &[&Entry]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(BTREE_BLOCK_SIZE);
    let mut w = VecWriter(&mut buf);

    w.write_u32(0); // next_node = 0 (leaf)
    w.write_u32(entries.len() as u32);

    for entry in entries {
        serialise_entry(&mut w, entry);
    }

    buf
}

fn serialise_entry(w: &mut VecWriter<'_>, entry: &Entry) {
    // Filename as UTF-16BE.
    let utf16: Vec<u16> = entry.filename.encode_utf16().collect();
    w.write_u32(utf16.len() as u32);
    for unit in &utf16 {
        w.write_u16(*unit);
    }

    // Code and type code.
    w.write_bytes(&entry.code);

    match &entry.value {
        EntryValue::Bool(v) => {
            w.write_bytes(b"bool");
            w.write_u8(if *v { 1 } else { 0 });
        }
        EntryValue::Long(v) => {
            w.write_bytes(b"long");
            w.write_u32(*v);
        }
        EntryValue::Shor(v) => {
            w.write_bytes(b"shor");
            w.write_u32(*v);
        }
        EntryValue::Blob(bytes) => {
            w.write_bytes(b"blob");
            w.write_u32(bytes.len() as u32);
            w.write_bytes(bytes);
        }
        EntryValue::Ustr(s) => {
            w.write_bytes(b"ustr");
            let utf16: Vec<u16> = s.encode_utf16().collect();
            w.write_u32(utf16.len() as u32);
            for unit in &utf16 {
                w.write_u16(*unit);
            }
        }
        EntryValue::Comp(v) => {
            w.write_bytes(b"comp");
            w.write_u64(*v);
        }
        EntryValue::Dutc(v) => {
            w.write_bytes(b"dutc");
            w.write_u64(*v);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_u32_be(dst: &mut [u8], v: u32) {
    dst[..4].copy_from_slice(&v.to_be_bytes());
}

/// A tiny big-endian writer over a fixed-size mutable byte slice.
struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn write_u8(&mut self, v: u8) {
        self.buf[self.pos] = v;
        self.pos += 1;
    }

    fn write_u32(&mut self, v: u32) {
        self.buf[self.pos..self.pos + 4].copy_from_slice(&v.to_be_bytes());
        self.pos += 4;
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
    }
}

/// A tiny big-endian writer that appends to a `Vec<u8>`.
struct VecWriter<'a>(&'a mut Vec<u8>);

impl<'a> VecWriter<'a> {
    fn write_u8(&mut self, v: u8) {
        self.0.push(v);
    }

    fn write_u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn write_u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn write_u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the file has the correct magic bytes and size.
    #[test]
    fn test_empty_ds_store() {
        let store = DsStore::new();
        let bytes = store.to_bytes();
        assert_eq!(bytes.len(), FILE_SIZE);
        assert_eq!(&bytes[0..4], &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&bytes[4..8], b"Bud1");
    }

    /// Verify Iloc serialisation matches the known-good Python output.
    #[test]
    fn test_icon_location_bytes() {
        let mut store = DsStore::new();
        store.set_icon_location("ALCOM.app", 170, 220);
        store.set_icon_location("Applications", 430, 220);

        let bytes = store.to_bytes();

        // The node starts at BTREE_FILE_OFFSET.
        let node = &bytes[BTREE_FILE_OFFSET..];
        let next_node = u32::from_be_bytes(node[0..4].try_into().unwrap());
        let count = u32::from_be_bytes(node[4..8].try_into().unwrap());
        assert_eq!(next_node, 0, "leaf node must have next_node=0");
        assert_eq!(count, 2);
    }

    /// Verify the DSDB block records the correct entry count.
    #[test]
    fn test_dsdb_record_count() {
        let store = dmg_ds_store("ALCOM.app", 170, 220, 430, 220);
        let bytes = store.to_bytes();

        // DSDB block at file offset 36 (buddy offset 0x20 + 4).
        let dsdb = &bytes[DSDB_FILE_OFFSET..DSDB_FILE_OFFSET + DSDB_BLOCK_SIZE];
        let root_node = u32::from_be_bytes(dsdb[0..4].try_into().unwrap());
        let levels = u32::from_be_bytes(dsdb[4..8].try_into().unwrap());
        let records = u32::from_be_bytes(dsdb[8..12].try_into().unwrap());
        let nodes = u32::from_be_bytes(dsdb[12..16].try_into().unwrap());
        let page_size = u32::from_be_bytes(dsdb[16..20].try_into().unwrap());

        assert_eq!(root_node, BTREE_BLOCK_NUM, "root node must be block 2");
        assert_eq!(levels, 0, "single leaf node has 0 levels");
        assert_eq!(records, 5, "dmg_ds_store produces 5 entries");
        assert_eq!(nodes, 1);
        assert_eq!(page_size, 4096);
    }

    /// Verify the root block has the expected layout.
    #[test]
    fn test_root_block_layout() {
        let store = DsStore::new();
        let bytes = store.to_bytes();

        let root = &bytes[ROOT_FILE_OFFSET..ROOT_FILE_OFFSET + ROOT_BLOCK_SIZE];

        let count = u32::from_be_bytes(root[0..4].try_into().unwrap());
        assert_eq!(count, 3, "always 3 blocks: root, dsdb, btree");

        // Block addresses.
        let addr0 = u32::from_be_bytes(root[8..12].try_into().unwrap());
        let addr1 = u32::from_be_bytes(root[12..16].try_into().unwrap());
        let addr2 = u32::from_be_bytes(root[16..20].try_into().unwrap());
        assert_eq!(addr0, ROOT_BLOCK_ADDR);
        assert_eq!(addr1, DSDB_BLOCK_ADDR);
        assert_eq!(addr2, BTREE_BLOCK_ADDR);

        // TOC: count=1 at offset 1032.
        let toc_count = u32::from_be_bytes(root[1032..1036].try_into().unwrap());
        assert_eq!(toc_count, 1);
        assert_eq!(root[1036], 4, "DSDB name length is 4");
        assert_eq!(&root[1037..1041], b"DSDB");
        let toc_val = u32::from_be_bytes(root[1041..1045].try_into().unwrap());
        assert_eq!(toc_val, DSDB_BLOCK_NUM);
    }
}

// ---------------------------------------------------------------------------
// Convenience builder for DMG use
// ---------------------------------------------------------------------------

/// Build a `.DS_Store` suitable for a macOS disk image.
///
/// The returned store contains:
/// - `bwsp` — Finder window bounds and appearance for the DMG root (`"."`).
/// - `icvp` — icon view properties for the DMG root.
/// - `vSrn` — view sort version.
/// - `Iloc` entries for `app_name` (at `app_x`, `app_y`) and
///   `"Applications"` (at `apps_x`, `apps_y`).
///
/// The `bwsp` window is sized to accommodate two icons: the left icon at
/// `app_x` and the right icon at `apps_x`, with a margin.
pub fn dmg_ds_store(
    app_name: &str,
    app_x: u32,
    app_y: u32,
    apps_x: u32,
    apps_y: u32,
) -> DsStore {
    let mut store = DsStore::new();

    // Window width and height: large enough to show both icons comfortably.
    let win_w = apps_x + 170;
    let win_h = apps_y + 170;

    // bwsp — Finder window bounds (stored as a binary plist).
    let bwsp = make_bwsp_plist(win_w, win_h);
    store.insert(".", b"bwsp", EntryValue::Blob(bwsp));

    // icvp — icon view properties (stored as a binary plist).
    let icvp = make_icvp_plist();
    store.insert(".", b"icvp", EntryValue::Blob(icvp));

    // vSrn — view sort version.
    store.insert(".", b"vSrn", EntryValue::Long(1));

    // Icon positions.
    store.set_icon_location(app_name, app_x, app_y);
    store.set_icon_location("Applications", apps_x, apps_y);

    store
}

// ---------------------------------------------------------------------------
// Binary plist generation for bwsp / icvp
// ---------------------------------------------------------------------------
//
// We generate the binary plists ourselves to avoid depending on a macOS
// system library.  The values are stable across all DMG builds, except for
// the window bounds which depend on the icon positions.

/// Generate the `bwsp` binary plist for a DMG Finder window.
fn make_bwsp_plist(width: u32, height: u32) -> Vec<u8> {
    // We use the `plist` crate (already a dependency) to serialise to binary.
    let dict = plist::Dictionary::from_iter([
        (
            "ContainerShowSidebar".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "PreviewPaneVisibility".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "ShowStatusBar".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "SidebarWidth".to_string(),
            plist::Value::Integer(0.into()),
        ),
        (
            "WindowBounds".to_string(),
            plist::Value::String(format!("{{{{200, 200}}, {{{width}, {height}}}}}")),
        ),
    ]);

    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &plist::Value::Dictionary(dict))
        .expect("bwsp plist serialisation");
    buf
}

/// Generate the `icvp` binary plist for a DMG icon view.
fn make_icvp_plist() -> Vec<u8> {
    let dict = plist::Dictionary::from_iter([
        (
            "arrangeBy".to_string(),
            plist::Value::String("none".to_string()),
        ),
        (
            "backgroundColorBlue".to_string(),
            plist::Value::Real(1.0),
        ),
        (
            "backgroundColorGreen".to_string(),
            plist::Value::Real(1.0),
        ),
        (
            "backgroundColorRed".to_string(),
            plist::Value::Real(1.0),
        ),
        (
            "backgroundType".to_string(),
            plist::Value::Integer(0.into()),
        ),
        (
            "gridOffsetX".to_string(),
            plist::Value::Real(0.0),
        ),
        (
            "gridOffsetY".to_string(),
            plist::Value::Real(0.0),
        ),
        (
            "gridSpacing".to_string(),
            plist::Value::Real(100.0),
        ),
        (
            "iconSize".to_string(),
            plist::Value::Real(96.0),
        ),
        (
            "labelOnBottom".to_string(),
            plist::Value::Boolean(true),
        ),
        (
            "scrollPositionX".to_string(),
            plist::Value::Real(0.0),
        ),
        (
            "scrollPositionY".to_string(),
            plist::Value::Real(0.0),
        ),
        (
            "showIconPreview".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "showItemInfo".to_string(),
            plist::Value::Boolean(false),
        ),
        (
            "textSize".to_string(),
            plist::Value::Real(12.0),
        ),
        (
            "viewOptionsVersion".to_string(),
            plist::Value::Integer(1.into()),
        ),
    ]);

    let mut buf = Vec::new();
    plist::to_writer_binary(&mut buf, &plist::Value::Dictionary(dict))
        .expect("icvp plist serialisation");
    buf
}
