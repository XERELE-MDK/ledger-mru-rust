//! Sync through a shared folder (Syncthing, Nextcloud, a USB stick…).
//!
//! **One file per device**: each device writes only `ledger-<device>.json` and
//! merges everybody else's `ledger-*.json`. Nobody ever writes the same file,
//! so the sync tool never sees a conflict — and because [`Book::merge`] is
//! commutative and idempotent, the order files arrive in does not matter.
//! The Android app follows the same layout (lib/sync.dart).

use std::path::PathBuf;
use std::time::SystemTime;

use crate::{is_demo_entry, Book, MergeReport};

pub const PREFIX: &str = "ledger-";
pub const SUFFIX: &str = ".json";

/// File name for a device id: `ledger-desktop-3fa2c1.json`.
pub fn device_file_name(device_id: &str) -> String {
    format!("{PREFIX}{device_id}{SUFFIX}")
}

/// Is `name` another device's ledger (or a sync-conflict copy of one)?
pub fn is_peer_file(name: &str, own: &str) -> bool {
    name.starts_with(PREFIX) && name.ends_with(SUFFIX) && name != own
}

/// Conflict copies made by Syncthing (`….sync-conflict-YYYYMMDD-…`).
pub fn is_conflict_copy(name: &str) -> bool {
    name.contains(".sync-conflict-")
}

/// Adds `b` into `a`.
pub fn add_reports(a: &mut MergeReport, b: &MergeReport) {
    a.entries_added += b.entries_added;
    a.entries_removed += b.entries_removed;
    a.accounts_added += b.accounts_added;
    a.accounts_updated += b.accounts_updated;
    a.accounts_removed += b.accounts_removed;
    a.accounts_kept_in_use += b.accounts_kept_in_use;
    a.conflicts += b.conflicts;
}

#[derive(Debug)]
pub struct PeerFile {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub book: Result<Book, String>,
}

/// Name, date and size of every peer file, sorted.
pub type PeersStamp = Vec<(String, SystemTime, u64)>;

/// A device's view of the shared folder.
#[derive(Clone, Debug)]
pub struct FolderSync {
    pub dir: PathBuf,
    pub device_id: String,
}

/// Result of one sync pass.
#[derive(Debug, Default)]
pub struct SyncOutcome {
    pub report: MergeReport,
    /// Other devices whose file was read.
    pub peers: Vec<String>,
    /// Files that could not be read (name, reason) — never fatal.
    pub skipped: Vec<(String, String)>,
}

impl FolderSync {
    pub fn new(dir: impl Into<PathBuf>, device_id: impl Into<String>) -> Self {
        Self { dir: dir.into(), device_id: device_id.into() }
    }

    pub fn own_name(&self) -> String {
        device_file_name(&self.device_id)
    }

    pub fn own_path(&self) -> PathBuf {
        self.dir.join(self.own_name())
    }

    /// Other devices' files currently in the folder.
    pub fn peers(&self) -> Result<Vec<PeerFile>, String> {
        let own = self.own_name();
        let rd = std::fs::read_dir(&self.dir).map_err(|e| format!("Cannot open {}: {e}", self.dir.display()))?;
        let mut out = Vec::new();
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().into_owned();
            if !is_peer_file(&name, &own) {
                continue;
            }
            let modified = ent.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
            let book = std::fs::read_to_string(ent.path()).map_err(|e| e.to_string()).and_then(|s| Book::from_json(&s));
            out.push(PeerFile { path: ent.path(), name, modified, book });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Fingerprint of the peer files (cheap change detection); `None` when the
    /// folder cannot be listed. Every file counts, not just the newest: the sync
    /// tool keeps original dates, so a file can arrive late and still be older.
    pub fn peers_stamp(&self) -> Option<PeersStamp> {
        let own = self.own_name();
        let mut v: PeersStamp = std::fs::read_dir(&self.dir)
            .ok()?
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let m = e.metadata().ok().filter(|_| is_peer_file(&name, &own))?;
                Some((name, m.modified().ok()?, m.len()))
            })
            .collect();
        v.sort();
        Some(v)
    }

    /// `book` as it should join this folder: without the demo entries that only
    /// this device has, when other devices already keep data here — the demo
    /// must not end up in their ledgers. Returns how many were left out.
    pub fn without_local_demo(&self, book: &Book) -> Result<(Book, usize), String> {
        let peers: Vec<Book> = self.peers()?.into_iter().filter_map(|p| p.book.ok()).collect();
        let mut out = book.clone();
        if !peers.is_empty() {
            out.entries.retain(|e| !is_demo_entry(&e.id) || peers.iter().any(|p| p.entries.iter().any(|x| x.id == e.id)));
        }
        let n = book.entries.len() - out.entries.len();
        Ok((out, n))
    }

    /// What merging the folder into `book` would change, without writing anything.
    pub fn preview(&self, book: &Book) -> Result<SyncOutcome, String> {
        let mut acc = book.clone();
        let mut out = SyncOutcome::default();
        for p in self.peers()? {
            match p.book {
                Ok(b) => {
                    let (m, r) = acc.merge(&b);
                    add_reports(&mut out.report, &r);
                    acc = m;
                    out.peers.push(p.name);
                }
                Err(e) => out.skipped.push((p.name, e)),
            }
        }
        Ok(out)
    }

    /// Merges every peer file into `book`, writes this device's file, and
    /// removes conflict copies once they are merged.
    pub fn sync(&self, book: &mut Book) -> Result<SyncOutcome, String> {
        let mut out = SyncOutcome::default();
        for p in self.peers()? {
            match p.book {
                Ok(b) => {
                    let (m, r) = book.merge(&b);
                    add_reports(&mut out.report, &r);
                    *book = m;
                    if is_conflict_copy(&p.name) {
                        let _ = std::fs::remove_file(&p.path);
                    }
                    out.peers.push(p.name);
                }
                Err(e) => out.skipped.push((p.name, e)),
            }
        }
        self.write_own(book)?;
        Ok(out)
    }

    /// Writes this device's file atomically (temp file + rename), so the sync
    /// tool never ships a half-written file — and only when the content differs:
    /// a new date on the same content would make every other device sync and
    /// rewrite its own file in turn, endlessly.
    pub fn write_own(&self, book: &Book) -> Result<(), String> {
        let json = book.checked_json()?;
        if std::fs::read_to_string(self.own_path()).is_ok_and(|on_disk| on_disk == json) {
            return Ok(());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        // Dot-prefixed: Syncthing and our own scan both ignore it.
        let tmp = self.dir.join(format!(".{}.tmp", self.own_name()));
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.own_path()).map_err(|e| e.to_string())
    }
}

/// A short random device id, e.g. `desktop-3fa2c1`.
pub fn new_device_id(platform: &str) -> String {
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mix = (nanos as u64) ^ ((std::process::id() as u64) << 20) ^ (nanos >> 64) as u64;
    format!("{platform}-{:06x}", mix.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40 & 0xff_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_entry_id, Kind, NewEntry};

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ledger-folder-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn add(b: &mut Book, memo: &str, amount: &str) -> String {
        let f = NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "", cr: "", amount, date: "2026-11-20", memo };
        b.add_entry(&f, new_entry_id()).unwrap()
    }

    #[test]
    fn two_devices_converge_through_one_folder() {
        let dir = tmpdir("converge");
        let base = Book::seed("2026-11-21");
        let (pc, phone) = (FolderSync::new(&dir, "desktop-aaaaaa"), FolderSync::new(&dir, "android-bbbbbb"));
        let (mut pc_book, mut phone_book) = (base.clone(), base.clone());

        // Each device works offline…
        let from_pc = add(&mut pc_book, "Bread", "50");
        let from_phone = add(&mut phone_book, "Taxi", "300");
        let victim = phone_book.entries[2].id.clone();
        phone_book.delete_entry(&victim);

        // …then the folder carries their files back and forth (order irrelevant).
        pc.sync(&mut pc_book).unwrap(); // writes pc file, no peer yet
        let o = phone.sync(&mut phone_book).unwrap();
        assert_eq!((o.report.entries_added, o.peers.len()), (1, 1));
        let o = pc.sync(&mut pc_book).unwrap();
        assert_eq!((o.report.entries_added, o.report.entries_removed), (1, 1));

        for b in [&pc_book, &phone_book] {
            assert!(b.entries.iter().any(|e| e.id == from_pc));
            assert!(b.entries.iter().any(|e| e.id == from_phone));
            assert!(!b.entries.iter().any(|e| e.id == victim));
        }
        // Nothing left to do on either side.
        assert!(pc.sync(&mut pc_book).unwrap().report.is_noop());
        assert!(phone.sync(&mut phone_book).unwrap().report.is_noop());
        // Exactly one file per device, no temp files left behind.
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().into_string().unwrap()).collect();
        names.sort();
        assert_eq!(names, ["ledger-android-bbbbbb.json", "ledger-desktop-aaaaaa.json"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflict_copies_are_merged_then_removed_and_bad_files_skipped() {
        let dir = tmpdir("conflict");
        let me = FolderSync::new(&dir, "desktop-cccccc");
        let mut mine = Book::seed("2026-11-21");
        let mut other = mine.clone();
        let id = add(&mut other, "Fuel", "900");
        std::fs::write(dir.join("ledger-android-dd.sync-conflict-20261121-101010-ABCDEFG.json"), other.to_json()).unwrap();
        std::fs::write(dir.join("ledger-broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        let o = me.sync(&mut mine).unwrap();
        assert!(mine.entries.iter().any(|e| e.id == id));
        assert_eq!(o.skipped.len(), 1);
        assert!(!std::fs::read_dir(&dir).unwrap().flatten().any(|e| is_conflict_copy(&e.file_name().to_string_lossy())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn set_mtime(path: &std::path::Path, secs: u64) {
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)).unwrap();
    }

    /// Nothing edited → nothing written. Rewriting an identical file changed
    /// its date, which made the other device rewrite its own, and so on.
    #[test]
    fn idle_devices_stop_writing() {
        let dir = tmpdir("idle");
        let base = Book::seed("2026-11-21");
        let devices = [FolderSync::new(&dir, "desktop-aaaaaa"), FolderSync::new(&dir, "android-bbbbbb")];
        let mut books = [base.clone(), base.clone()];
        add(&mut books[0], "Bread", "50");
        add(&mut books[1], "Taxi", "300");
        let pass = |books: &mut [Book; 2]| {
            for (d, b) in devices.iter().zip(books.iter_mut()) {
                d.sync(b).unwrap();
            }
        };
        pass(&mut books);
        pass(&mut books);
        for d in &devices {
            set_mtime(&d.own_path(), 1_000_000); // any write would change it
        }
        for _ in 0..5 {
            pass(&mut books);
        }
        for d in &devices {
            let m = std::fs::metadata(d.own_path()).unwrap().modified().unwrap();
            assert_eq!(m, SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000), "{}", d.device_id);
        }
        // A real edit is written, and so is a file that went missing or was altered.
        add(&mut books[0], "Fuel", "900");
        devices[0].write_own(&books[0]).unwrap();
        assert!(std::fs::read_to_string(devices[0].own_path()).unwrap().contains("Fuel"));
        std::fs::remove_file(devices[0].own_path()).unwrap();
        devices[0].sync(&mut books[0]).unwrap();
        assert!(devices[0].own_path().exists());
        std::fs::write(devices[0].own_path(), "{ cut sho").unwrap();
        devices[0].sync(&mut books[0]).unwrap();
        assert!(Book::load_or_seed(&devices[0].own_path()).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Syncthing keeps each file's original date: with three devices, a file can
    /// arrive late and still be older than another one. It must be noticed.
    #[test]
    fn a_late_but_older_file_is_noticed() {
        let dir = tmpdir("late");
        let me = FolderSync::new(&dir, "desktop-aaaaaa");
        let json = Book::seed("2026-11-21").to_json();
        let (b, c) = (dir.join("ledger-android-b.json"), dir.join("ledger-linux-c.json"));
        std::fs::write(&b, &json).unwrap();
        std::fs::write(&c, &json).unwrap();
        set_mtime(&b, 2_000);
        set_mtime(&c, 9_000);
        let seen = me.peers_stamp();
        assert_eq!(me.peers_stamp(), seen);
        set_mtime(&b, 5_000);
        assert_ne!(me.peers_stamp(), seen, "b changed, though c is still the newest");
        let seen = me.peers_stamp();
        me.write_own(&Book::seed("2026-11-21")).unwrap();
        assert_eq!(me.peers_stamp(), seen, "our own file does not count");
        std::fs::remove_file(&b).unwrap();
        assert_ne!(me.peers_stamp(), seen);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undoing_a_deletion_survives_sync() {
        let dir = tmpdir("undo");
        let base = Book::seed("2026-11-21");
        let (pc, phone) = (FolderSync::new(&dir, "desktop-aaaaaa"), FolderSync::new(&dir, "android-bbbbbb"));
        let (mut pc_book, mut phone_book) = (base.clone(), base.clone());
        pc.sync(&mut pc_book).unwrap();
        phone.sync(&mut phone_book).unwrap();
        let victim = pc_book.entries[3].clone();
        let e = pc_book.delete_entry(&victim.id).unwrap();
        pc.write_own(&pc_book).unwrap();
        phone.sync(&mut phone_book).unwrap(); // the deletion has travelled…
        pc_book.restore_entry(e); // …when the user hits Undo
        for _ in 0..3 {
            pc.sync(&mut pc_book).unwrap();
            phone.sync(&mut phone_book).unwrap();
        }
        for b in [&pc_book, &phone_book] {
            assert_eq!(b.entries.len(), base.entries.len());
            assert_eq!(b.entries.iter().filter(|x| x.memo == victim.memo && x.date == victim.date && x.lines == victim.lines).count(), 1);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn joining_a_folder_does_not_bring_the_demo() {
        let dir = tmpdir("demo");
        let phone = FolderSync::new(&dir, "android-bbbbbb");
        let pc = FolderSync::new(&dir, "desktop-aaaaaa");
        let mut mine = Book::seed("2026-11-25");
        // First device in an empty folder: nothing to protect, nothing dropped.
        assert_eq!(pc.without_local_demo(&mine).unwrap().1, 0);

        // The phone keeps real data there, plus one demo entry it made its own.
        let mut real = Book::seed("2026-11-21");
        real.entries.truncate(1);
        add(&mut real, "Real", "50");
        phone.sync(&mut real).unwrap();
        std::fs::write(dir.join("ledger-broken.json"), "{ not json").unwrap();

        let demo = mine.entries.len();
        add(&mut mine, "Typed here", "7");
        mine.entries.push(real.entries[0].clone());
        let (mut joined, n) = pc.without_local_demo(&mine).unwrap();
        assert_eq!((n, joined.entries.len()), (demo, 2));
        assert!(joined.deleted_entries.is_empty(), "left out, not deleted everywhere");
        pc.sync(&mut joined).unwrap();
        phone.sync(&mut real).unwrap();
        for b in [&joined, &real] {
            let mut memos: Vec<&str> = b.entries.iter().map(|e| e.memo.as_str()).collect();
            memos.sort();
            assert_eq!(memos, ["Opening balances", "Real", "Typed here"]);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names() {
        assert!(is_peer_file("ledger-android-1.json", "ledger-desktop-2.json"));
        assert!(!is_peer_file("ledger-desktop-2.json", "ledger-desktop-2.json"));
        assert!(!is_peer_file(".ledger-desktop-2.json.tmp", "x"));
        assert!(!is_peer_file("ledger-mru-export.json.bak", "x"));
        let id = new_device_id("desktop");
        assert!(id.starts_with("desktop-") && id.len() == 14, "{id}");
    }
}
