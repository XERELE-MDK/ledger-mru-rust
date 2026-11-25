//! ledger-core — double-entry personal ledger in MRU.
//!
//! Pure logic, no UI. The JSON format is the one used by the Android app
//! (`ledger_mru`, key `ledger-mru-v1`) so a file exported from one can be
//! imported by the other.

pub mod folder;
pub mod format;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use format::*;

// ───────────────────────── Model ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

impl AccountType {
    pub const ORDER: [AccountType; 5] = [
        AccountType::Asset,
        AccountType::Liability,
        AccountType::Equity,
        AccountType::Income,
        AccountType::Expense,
    ];
    pub fn rank(self) -> usize {
        Self::ORDER.iter().position(|t| *t == self).unwrap()
    }
    /// True when the account's balance normally sits on the debit side.
    pub fn debit_normal(self) -> bool {
        matches!(self, AccountType::Asset | AccountType::Expense)
    }
    pub fn label(self) -> &'static str {
        ["Asset", "Liability", "Equity", "Income", "Expense"][self.rank()]
    }
    pub fn group_label(self) -> &'static str {
        ["Assets", "Liabilities", "Equity", "Income", "Expenses"][self.rank()]
    }
    pub fn hint(self) -> &'static str {
        ["What I have", "What I owe", "Starting point", "Money coming in", "Money going out"][self.rank()]
    }
    pub fn key(self) -> &'static str {
        ["asset", "liability", "equity", "income", "expense"][self.rank()]
    }
    pub fn from_key(k: &str) -> Option<Self> {
        Self::ORDER.into_iter().find(|t| t.key() == k)
    }
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: AccountType,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub budget: f64,
    /// Last edit, in ms since the epoch (0 = never edited / demo). Newest wins on merge.
    #[serde(default, rename = "updatedAt", skip_serializing_if = "is_zero_i64")]
    pub updated_at: i64,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// "This id was deleted at …" — lets a deletion travel to the other device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tombstone {
    pub id: String,
    /// ms since the epoch
    pub at: i64,
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}

/// A new, practically unique entry id: time + a few random-ish bits.
pub fn new_entry_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let salt = (nanos ^ std::process::id().rotate_left(16)) & 0xffff;
    format!("e{}-{salt:04x}", now_ms())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// Account id.
    pub a: String,
    pub dr: f64,
    pub cr: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    /// "YYYY-MM-DD"
    pub date: String,
    pub memo: String,
    pub lines: Vec<Line>,
}

impl Entry {
    pub fn amount(&self) -> f64 {
        self.lines.iter().map(|l| l.dr).sum()
    }
    pub fn is_balanced(&self) -> bool {
        let dr: f64 = self.lines.iter().map(|l| l.dr).sum();
        let cr: f64 = self.lines.iter().map(|l| l.cr).sum();
        (dr - cr).abs() < 0.005
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Simple,
    Pro,
}

/// Light / dark preference, shared with the Android app ("theme" key).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Book {
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default = "yes")]
    pub hints: bool,
    #[serde(default)]
    pub khoums: bool,
    #[serde(default)]
    pub theme: ThemePref,
    #[serde(default, rename = "deletedEntries", skip_serializing_if = "Vec::is_empty")]
    pub deleted_entries: Vec<Tombstone>,
    #[serde(default, rename = "deletedAccounts", skip_serializing_if = "Vec::is_empty")]
    pub deleted_accounts: Vec<Tombstone>,
}

/// Is this one of the demo entries ("s20261125-12", see [`Book::seed`])?
pub fn is_demo_entry(id: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    matches!(id.strip_prefix('s').and_then(|r| r.split_once('-')), Some((day, n)) if day.len() == 8 && digits(day) && digits(n))
}

/// How an entry reads in simple mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Expense,
    Income,
    Transfer,
}

#[derive(Clone, Debug)]
pub struct EntryView {
    pub kind: Kind,
    pub title: String,
    pub sub: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Totals {
    pub inc: f64,
    pub exp: f64,
}

// ───────────────────────── Persistence ─────────────────────────

impl Book {
    pub fn from_json(s: &str) -> Result<Book, String> {
        let b: Book = serde_json::from_str(s).map_err(|e| format!("Invalid ledger file: {e}"))?;
        if let Some(e) = b.entries.iter().find(|e| !e.is_balanced()) {
            return Err(format!("Entry \"{}\" ({}) is not balanced.", e.memo, e.date));
        }
        if let Some(e) = b.entries.iter().find(|e| e.lines.iter().any(|l| b.account(&l.a).is_none())) {
            return Err(format!("Entry \"{}\" uses an unknown account.", e.memo));
        }
        Ok(b)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("ledger serializes")
    }

    /// Loads the book at `path`, or the demo data when the file does not exist yet.
    pub fn load_or_seed(path: &Path) -> Result<Book, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Book::from_json(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Book::seed(&today())),
            Err(e) => Err(format!("Cannot read {}: {e}", path.display())),
        }
    }

    /// JSON that is known to load again. Never replace a readable file with
    /// one that could not be read back.
    pub(crate) fn checked_json(&self) -> Result<String, String> {
        let json = self.to_json();
        Book::from_json(&json)?;
        Ok(json)
    }

    /// Atomic save: write a temp file next to the target, then rename.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = self.checked_json()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}

/// Moves a ledger file that does not load out of the way, untouched:
/// `ledger.json` → `ledger.damaged-20261125-174500.json`. Returns the new path.
///
/// Only the content may be bad: when the file itself cannot be read (missing,
/// permissions, failing disk…) it is left where it is and this returns an error.
pub fn set_aside(path: &Path) -> Result<PathBuf, String> {
    std::fs::read(path).map_err(|e| e.to_string())?;
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let mut copy = path.with_file_name(format!("{stem}.damaged-{stamp}.json"));
    let mut k = 2;
    while copy.exists() {
        copy = path.with_file_name(format!("{stem}.damaged-{stamp}-{k}.json"));
        k += 1;
    }
    std::fs::rename(path, &copy).map_err(|e| e.to_string())?;
    Ok(copy)
}

// ───────────────────────── Demo data ─────────────────────────

impl Book {
    /// Three months of a realistic household, ending at `today` ("YYYY-MM-DD").
    pub fn seed(today: &str) -> Book {
        use AccountType::*;
        let acc = |id: &str, name: &str, kind, budget| Account { id: id.into(), name: name.into(), kind, budget, updated_at: 0 };
        let accounts = vec![
            acc("cash", "Cash", Asset, 0.0),
            acc("bank", "Bank account", Asset, 0.0),
            acc("wallet", "Mobile wallet", Asset, 0.0),
            acc("savings", "Savings", Asset, 0.0),
            acc("loan", "Money I owe", Liability, 0.0),
            acc("opening", "Opening balance", Equity, 0.0),
            acc("salary", "Salary", Income, 0.0),
            acc("side", "Side work", Income, 0.0),
            acc("rent", "Rent", Expense, 8000.0),
            acc("food", "Groceries", Expense, 5000.0),
            acc("transport", "Transport", Expense, 1500.0),
            acc("utilities", "Water & electricity", Expense, 1200.0),
            acc("phone", "Phone & internet", Expense, 600.0),
            acc("eating", "Eating out", Expense, 1500.0),
            acc("family", "Family support", Expense, 3000.0),
            acc("health", "Health", Expense, 800.0),
        ];
        let ym0 = &today[..7];
        let day_today: u32 = today[8..10].parse().unwrap_or(28);
        let mut entries = Vec::new();
        let mut n = 0;
        let tag = today.replace('-', "");
        let mut push = |date: String, memo: &str, lines: Vec<Line>| {
            entries.push(Entry { id: format!("s{tag}-{n}"), date, memo: memo.into(), lines });
            n += 1;
        };
        let l = |a: &str, dr: f64, cr: f64| Line { a: a.into(), dr, cr };
        push(
            format!("{}-01", ym_shift(ym0, -2)),
            "Opening balances",
            vec![l("cash", 3000.0, 0.0), l("bank", 21000.0, 0.0), l("wallet", 1500.0, 0.0), l("opening", 0.0, 25500.0)],
        );
        let plan: [(u32, &str, &str, &str, [f64; 3]); 21] = [
            (2, "Monthly rent", "rent", "bank", [8000., 8000., 8000.]),
            (3, "ATM withdrawal", "cash", "bank", [4000., 4500., 4000.]),
            (4, "Salary", "bank", "salary", [25000., 25000., 25000.]),
            (5, "Electricity & water bill", "utilities", "wallet", [1080., 1150., 1240.]),
            (6, "Phone & internet plan", "phone", "wallet", [500., 500., 500.]),
            (6, "Move to savings", "savings", "bank", [3000., 3000., 2500.]),
            (7, "Weekly groceries", "food", "cash", [1150., 1320., 1260.]),
            (8, "Taxi", "transport", "cash", [300., 250., 400.]),
            (10, "Sent to family", "family", "bank", [3000., 3000., 3000.]),
            (12, "Lunch with friends", "eating", "cash", [450., 600., 380.]),
            (14, "Weekly groceries", "food", "cash", [1240., 1100., 1380.]),
            (15, "Wallet top-up", "wallet", "bank", [2000., 2000., 2000.]),
            (16, "Taxi", "transport", "wallet", [350., 400., 300.]),
            (17, "Repaid brother (part)", "loan", "wallet", [0., 0., 1000.]),
            (18, "Pharmacy", "health", "cash", [0., 320., 950.]),
            (19, "Website job", "wallet", "side", [0., 4000., 2500.]),
            (21, "Weekly groceries", "food", "cash", [1180., 1260., 1200.]),
            (23, "Dinner out", "eating", "wallet", [700., 520., 600.]),
            (24, "Borrowed from brother", "cash", "loan", [0., 2000., 0.]),
            (26, "Taxi", "transport", "cash", [280., 350., 300.]),
            (28, "Weekly groceries", "food", "cash", [1300., 1210., 1250.]),
        ];
        for (i, off) in [-2, -1, 0].into_iter().enumerate() {
            let m = ym_shift(ym0, off);
            for (d, memo, dr, cr, amts) in plan {
                let amt = amts[i];
                if amt == 0.0 || (off == 0 && d > day_today) {
                    continue;
                }
                push(format!("{m}-{d:02}"), memo, vec![l(dr, amt, 0.0), l(cr, 0.0, amt)]);
            }
        }
        Book {
            accounts,
            entries,
            mode: Mode::Simple,
            hints: true,
            khoums: false,
            theme: ThemePref::System,
            deleted_entries: vec![],
            deleted_accounts: vec![],
        }
    }
}

// ───────────────────────── Queries ─────────────────────────

impl Book {
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }
    pub fn name_of(&self, id: &str) -> String {
        self.account(id).map_or_else(|| "(deleted)".into(), |a| a.name.clone())
    }
    pub fn type_of(&self, id: &str) -> Option<AccountType> {
        self.account(id).map(|a| a.kind)
    }
    pub fn fmt(&self, n: f64) -> String {
        f2(n, self.khoums)
    }

    /// Accounts ordered asset → liability → equity → income → expense (stable).
    pub fn sorted_accounts(&self) -> Vec<&Account> {
        let mut v: Vec<&Account> = self.accounts.iter().collect();
        v.sort_by_key(|a| a.kind.rank()); // stable
        v
    }

    /// All entries by date, insertion order kept on equal dates.
    pub fn sorted_entries(&self) -> Vec<&Entry> {
        let mut v: Vec<&Entry> = self.entries.iter().collect();
        v.sort_by(|a, b| a.date.cmp(&b.date)); // stable
        v
    }

    pub fn entries_in(&self, ym: &str) -> Vec<&Entry> {
        self.sorted_entries().into_iter().filter(|e| e.date.starts_with(ym)).collect()
    }

    pub fn totals<'a>(&self, es: impl IntoIterator<Item = &'a Entry>) -> Totals {
        let types: HashMap<&str, AccountType> = self.accounts.iter().map(|a| (a.id.as_str(), a.kind)).collect();
        let (mut inc, mut exp) = (0.0, 0.0);
        for e in es {
            for l in &e.lines {
                match types.get(l.a.as_str()) {
                    Some(AccountType::Income) => inc += l.cr - l.dr,
                    Some(AccountType::Expense) => exp += l.dr - l.cr,
                    _ => {}
                }
            }
        }
        Totals { inc, exp }
    }

    /// Raw debit-minus-credit balance of `id` up to and including date `to`.
    pub fn raw_bal(&self, id: &str, to: &str) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.date.as_str() <= to)
            .flat_map(|e| &e.lines)
            .filter(|l| l.a == id)
            .map(|l| l.dr - l.cr)
            .sum()
    }

    /// Balance shown on the account's normal side.
    pub fn normal_bal(a: &Account, raw: f64) -> f64 {
        if a.kind.debit_normal() { raw } else { -raw }
    }

    /// Spending per expense account in `ym`, largest first, positive only.
    pub fn spending_in(&self, ym: &str) -> Vec<(String, String, f64)> {
        let mut spend: Vec<(String, f64)> = Vec::new();
        for e in self.entries_in(ym) {
            for l in &e.lines {
                if self.type_of(&l.a) == Some(AccountType::Expense) {
                    match spend.iter_mut().find(|(id, _)| *id == l.a) {
                        Some((_, v)) => *v += l.dr - l.cr,
                        None => spend.push((l.a.clone(), l.dr - l.cr)),
                    }
                }
            }
        }
        let mut v: Vec<_> = spend.into_iter().filter(|(_, a)| *a > 0.0).map(|(id, a)| {
            let name = self.name_of(&id);
            (id, name, a)
        }).collect();
        v.sort_by(|a, b| b.2.total_cmp(&a.2));
        v
    }

    pub fn describe(&self, e: &Entry) -> EntryView {
        let ty = |l: &Line| self.type_of(&l.a);
        if let Some(ex) = e.lines.iter().find(|l| ty(l) == Some(AccountType::Expense)) {
            let from = e.lines.iter().find(|l| l.cr > 0.0 && !std::ptr::eq(*l, ex));
            return EntryView {
                kind: Kind::Expense,
                title: self.name_of(&ex.a),
                sub: from.map(|o| format!("from {}", self.name_of(&o.a))).unwrap_or_default(),
            };
        }
        if let Some(inc) = e.lines.iter().find(|l| ty(l) == Some(AccountType::Income)) {
            let into = e.lines.iter().find(|l| l.dr > 0.0);
            return EntryView {
                kind: Kind::Income,
                title: self.name_of(&inc.a),
                sub: into.map(|o| format!("into {}", self.name_of(&o.a))).unwrap_or_default(),
            };
        }
        let d: Vec<_> = e.lines.iter().filter(|l| l.dr > 0.0).collect();
        let c: Vec<_> = e.lines.iter().filter(|l| l.cr > 0.0).collect();
        let title = if d.len() == 1 && c.len() == 1 {
            format!("{} → {}", self.name_of(&c[0].a), self.name_of(&d[0].a))
        } else {
            e.memo.clone()
        };
        EntryView { kind: Kind::Transfer, title, sub: "transfer".into() }
    }

    pub fn account_in_use(&self, id: &str) -> bool {
        self.entries.iter().any(|e| e.lines.iter().any(|l| l.a == id))
    }

    /// Category options for a simple-mode entry kind.
    pub fn category_options(&self, kind: Kind) -> Vec<&Account> {
        self.accounts
            .iter()
            .filter(|a| match kind {
                Kind::Expense => a.kind == AccountType::Expense,
                Kind::Income => a.kind == AccountType::Income,
                Kind::Transfer => matches!(a.kind, AccountType::Asset | AccountType::Liability),
            })
            .collect()
    }

    /// "Paid with" / "Received in" / "Move from" options.
    pub fn via_options(&self) -> Vec<&Account> {
        self.accounts
            .iter()
            .filter(|a| matches!(a.kind, AccountType::Asset | AccountType::Liability))
            .collect()
    }
}

// ───────────────────────── Mutations ─────────────────────────

/// A new entry as typed in the form.
pub struct NewEntry<'a> {
    pub kind: Kind,
    /// Simple mode: category / source / "move to".
    pub cat: &'a str,
    /// Simple mode: "paid with" / "received in" / "move from".
    pub via: &'a str,
    /// Accountant mode.
    pub dr: &'a str,
    pub cr: &'a str,
    pub amount: &'a str,
    pub date: &'a str,
    pub memo: &'a str,
}

impl Book {
    /// Debit and credit account ids for the form, per the current mode.
    pub fn sides<'a>(&self, f: &NewEntry<'a>) -> (&'a str, &'a str) {
        match (self.mode, f.kind) {
            (Mode::Pro, _) => (f.dr, f.cr),
            (Mode::Simple, Kind::Income) => (f.via, f.cat),
            (Mode::Simple, _) => (f.cat, f.via),
        }
    }

    /// Validates the form and appends the entry. Returns the new entry's id.
    pub fn add_entry(&mut self, f: &NewEntry, id: String) -> Result<String, String> {
        let amt = parse_amount(f.amount);
        if !(amt > 0.0) {
            return Err("Enter an amount above 0.".into());
        }
        let (dr, cr) = self.sides(f);
        if dr.is_empty() || cr.is_empty() || self.account(dr).is_none() || self.account(cr).is_none() {
            return Err("Pick both accounts.".into());
        }
        if dr == cr {
            return Err("Debit and credit must be different accounts.".into());
        }
        if parse_date(f.date).is_none() || f.date.len() != 10 {
            return Err("Enter a date as YYYY-MM-DD.".into());
        }
        let memo = match f.memo.trim() {
            "" => match (self.mode, f.kind) {
                (Mode::Pro, _) => format!("{} / {}", self.name_of(dr), self.name_of(cr)),
                (Mode::Simple, Kind::Transfer) => format!("Transfer to {}", self.name_of(dr)),
                (Mode::Simple, Kind::Income) => self.name_of(cr),
                (Mode::Simple, Kind::Expense) => self.name_of(dr),
            },
            m => m.to_string(),
        };
        self.entries.push(Entry {
            id: id.clone(),
            date: f.date.to_string(),
            memo,
            lines: vec![
                Line { a: dr.into(), dr: amt, cr: 0.0 },
                Line { a: cr.into(), dr: 0.0, cr: amt },
            ],
        });
        Ok(id)
    }

    /// Removes the entry and records a tombstone so the deletion syncs.
    pub fn delete_entry(&mut self, id: &str) -> Option<Entry> {
        let i = self.entries.iter().position(|e| e.id == id)?;
        self.deleted_entries.retain(|t| t.id != id);
        self.deleted_entries.push(Tombstone { id: id.into(), at: now_ms() });
        Some(self.entries.remove(i))
    }

    /// Undo of [`Book::delete_entry`]: the entry comes back under a new id.
    ///
    /// The tombstone stays. Another device may already have it, and a tombstone
    /// always wins: under its old id the entry would vanish again, everywhere.
    pub fn restore_entry(&mut self, e: Entry) -> String {
        let id = new_entry_id();
        self.entries.push(Entry { id: id.clone(), ..e });
        id
    }

    pub fn demo_count(&self) -> usize {
        self.entries.iter().filter(|e| is_demo_entry(&e.id)).count()
    }

    /// "Start with an empty book": removes the demo entries, and only those.
    /// The user's own entries, the accounts and their budgets stay. Tombstones
    /// make it hold when other devices have the same demo. Returns how many.
    pub fn remove_demo(&mut self) -> usize {
        let ids: Vec<String> = self.entries.iter().filter(|e| is_demo_entry(&e.id)).map(|e| e.id.clone()).collect();
        for id in &ids {
            self.delete_entry(id);
        }
        ids.len()
    }

    /// Creates (id = None) or updates an account.
    pub fn save_account(&mut self, id: Option<&str>, name: &str, kind: AccountType, budget: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Give the account a name.".into());
        }
        let budget = if kind == AccountType::Expense { parse_amount(budget).max(0.0) } else { 0.0 };
        match id.and_then(|id| self.accounts.iter_mut().find(|a| a.id == id)) {
            Some(a) => {
                a.name = name.into();
                a.kind = kind;
                a.budget = budget;
                a.updated_at = now_ms();
                Ok(a.id.clone())
            }
            None => {
                let slug: String = name
                    .to_lowercase()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect();
                let mut new_id = slug.trim_matches('-').to_string();
                if new_id.is_empty() {
                    new_id = "account".into();
                }
                let base = new_id.clone();
                let mut k = 2;
                while self.account(&new_id).is_some() {
                    new_id = format!("{base}-{k}");
                    k += 1;
                }
                self.deleted_accounts.retain(|t| t.id != new_id);
                self.accounts.push(Account { id: new_id.clone(), name: name.into(), kind, budget, updated_at: now_ms() });
                Ok(new_id)
            }
        }
    }

    pub fn delete_account(&mut self, id: &str) -> Result<(), String> {
        if self.account_in_use(id) {
            return Err("This account has entries. Delete those first.".into());
        }
        self.accounts.retain(|a| a.id != id);
        self.deleted_accounts.retain(|t| t.id != id);
        self.deleted_accounts.push(Tombstone { id: id.into(), at: now_ms() });
        Ok(())
    }
}

// ───────────────────────── Merge (sync) ─────────────────────────

/// What a merge changed, from the local book's point of view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MergeReport {
    pub entries_added: usize,
    pub entries_removed: usize,
    pub accounts_added: usize,
    pub accounts_updated: usize,
    pub accounts_removed: usize,
    /// Deleted on one side but still used by entries: kept.
    pub accounts_kept_in_use: usize,
    /// Same id, different content (should not happen: entries are immutable).
    pub conflicts: usize,
}

impl MergeReport {
    pub fn is_noop(&self) -> bool {
        *self == MergeReport::default()
    }
}

fn union_tombstones(a: &[Tombstone], b: &[Tombstone]) -> Vec<Tombstone> {
    let mut out: Vec<Tombstone> = a.to_vec();
    for t in b {
        match out.iter_mut().find(|x| x.id == t.id) {
            Some(x) => x.at = x.at.max(t.at),
            None => out.push(t.clone()),
        }
    }
    out
}

impl Book {
    /// Merges `other` into a copy of `self` — commutative on data, idempotent.
    ///
    /// * entries: union by id, minus tombstoned ids (entries are immutable);
    /// * accounts: union by id, newest `updatedAt` wins; an account tombstone
    ///   wins over older edits, unless entries still use the account;
    /// * settings (mode, hints, khoums, theme) stay local: they are per device.
    ///
    /// The Android app implements the same rules (lib/model.dart, `mergeFiles`).
    pub fn merge(&self, other: &Book) -> (Book, MergeReport) {
        let mut r = MergeReport::default();
        let deleted_entries = union_tombstones(&self.deleted_entries, &other.deleted_entries);
        let mut deleted_accounts = union_tombstones(&self.deleted_accounts, &other.deleted_accounts);
        let gone = |id: &str| deleted_entries.iter().any(|t| t.id == id);

        // entries
        let mut entries: Vec<Entry> = self.entries.iter().filter(|e| !gone(&e.id)).cloned().collect();
        r.entries_removed = self.entries.len() - entries.len();
        for e in &other.entries {
            match entries.iter().find(|x| x.id == e.id) {
                Some(x) => {
                    // The same demo entry in two languages is no conflict: each keeps its own.
                    if x != e && !is_demo_entry(&e.id) {
                        r.conflicts += 1;
                    }
                }
                None if !gone(&e.id) && !self.entries.iter().any(|x| x.id == e.id) => {
                    entries.push(e.clone());
                    r.entries_added += 1;
                }
                None => {}
            }
        }

        // accounts: newest version of each id
        let mut accounts: Vec<Account> = self.accounts.clone();
        for a in &other.accounts {
            match accounts.iter_mut().find(|x| x.id == a.id) {
                Some(x) if a.updated_at > x.updated_at => {
                    // Only a visible change counts (not a bare timestamp bump).
                    if (&x.name, x.kind, x.budget) != (&a.name, a.kind, a.budget) {
                        r.accounts_updated += 1;
                    }
                    *x = a.clone();
                }
                Some(x) => {
                    // 0 = never edited: the name is just a translation.
                    if a.updated_at == x.updated_at && a.updated_at != 0 && *x != *a {
                        r.conflicts += 1;
                    }
                }
                None => {
                    accounts.push(a.clone());
                    r.accounts_added += 1;
                }
            }
        }
        // account tombstones
        let used = |id: &str| entries.iter().any(|e| e.lines.iter().any(|l| l.a == id));
        accounts.retain_mut(|a| {
            let Some(t) = deleted_accounts.iter().find(|t| t.id == a.id) else { return true };
            if t.at < a.updated_at {
                return true; // edited (or re-created) after the deletion
            }
            if used(&a.id) {
                // Still referenced: keep it, and make the decision stick.
                a.updated_at = t.at + 1;
                r.accounts_kept_in_use += 1;
                return true;
            }
            false
        });
        r.accounts_removed = self.accounts.iter().filter(|a| !accounts.iter().any(|x| x.id == a.id)).count();
        // An account added by `other` and removed again by a tombstone was never "added".
        r.accounts_added -= other
            .accounts
            .iter()
            .filter(|a| !self.accounts.iter().any(|x| x.id == a.id) && !accounts.iter().any(|x| x.id == a.id))
            .count();
        deleted_accounts.retain(|t| !accounts.iter().any(|a| a.id == t.id && a.updated_at > t.at));

        let book = Book { accounts, entries, deleted_entries, deleted_accounts, ..self.clone() };
        (book, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Book {
        Book::seed("2026-11-21")
    }

    #[test]
    fn demo_entries_balance_and_trial_balance_is_even() {
        let b = demo();
        assert!(b.entries.iter().all(Entry::is_balanced));
        let sum: f64 = b.accounts.iter().map(|a| b.raw_bal(&a.id, "9999-12-31")).sum();
        assert!(sum.abs() < 0.01);
        // Same figures as the Android app on the same day.
        let t = b.totals(b.entries_in("2026-11"));
        assert_eq!((t.inc, t.exp), (27500.0, 18610.0));
        assert_eq!(b.raw_bal("cash", "2026-11-31"), -380.0);
    }

    #[test]
    fn simple_expense_and_income_write_the_right_sides() {
        let mut b = demo();
        let f = NewEntry { kind: Kind::Expense, cat: "transport", via: "cash", dr: "", cr: "", amount: "300", date: "2030-01-05", memo: "" };
        b.add_entry(&f, "t1".into()).unwrap();
        let e = b.entries.last().unwrap();
        assert_eq!((e.lines[0].a.as_str(), e.lines[1].a.as_str()), ("transport", "cash"));
        assert_eq!(e.memo, "Transport");
        let f = NewEntry { kind: Kind::Income, cat: "salary", via: "bank", amount: "1 000,5", ..f };
        b.add_entry(&f, "t2".into()).unwrap();
        let e = b.entries.last().unwrap();
        assert_eq!((e.lines[0].a.as_str(), e.lines[1].a.as_str(), e.amount()), ("bank", "salary", 1000.5));
        assert_eq!(b.describe(e).kind, Kind::Income);
    }

    #[test]
    fn validation_errors() {
        let mut b = demo();
        let f = NewEntry { kind: Kind::Transfer, cat: "cash", via: "cash", dr: "", cr: "", amount: "10", date: "2030-01-05", memo: "" };
        assert!(b.add_entry(&f, "x".into()).unwrap_err().contains("different"));
        let f = NewEntry { amount: "0", ..f };
        assert!(b.add_entry(&f, "x".into()).is_err());
        assert!(b.delete_account("cash").is_err());
    }

    #[test]
    fn json_round_trip_matches_android_format() {
        let b = demo();
        let s = b.to_json();
        // Regenerates the fixture used by the Android tests (ledger_mru/test/fixtures).
        if let Ok(p) = std::env::var("LEDGER_WRITE_FIXTURE") {
            std::fs::write(p, &s).unwrap();
        }
        assert!(s.contains("\"type\": \"asset\"") && s.contains("\"mode\": \"simple\""));
        let back = Book::from_json(&s).unwrap();
        assert_eq!(back.entries, b.entries);
        // A file written by the Android app (no hints/khoums keys) loads with defaults.
        let android = r#"{"accounts":[{"id":"cash","name":"Cash","type":"asset"}],"entries":[],"mode":"pro"}"#;
        let a = Book::from_json(android).unwrap();
        assert!(a.hints && !a.khoums && a.mode == Mode::Pro && a.theme == ThemePref::System);
        let dark = Book::from_json(&android.replace(r#""mode":"pro""#, r#""mode":"pro","theme":"dark""#)).unwrap();
        assert_eq!(dark.theme, ThemePref::Dark);
    }

    /// A real export from the Android app (written by its test suite) must load here.
    #[test]
    fn reads_android_export() {
        let raw = include_str!("../tests/fixtures/android_export.json");
        let b = Book::from_json(raw).expect("Android export parses");
        assert!(b.entries.len() > 40 && b.entries.iter().all(Entry::is_balanced));
        assert_eq!((b.mode, b.theme, b.khoums), (Mode::Pro, ThemePref::Dark, true));
        let e = b.entries.iter().find(|e| e.id == "android-test").expect("custom entry kept");
        assert_eq!((e.memo.as_str(), e.amount()), ("Café Nouakchott", 1250.5));
        assert_eq!(b.account("fuel").map(|a| a.budget), Some(2000.0));
        assert!(b.account("fuel").unwrap().updated_at > 0, "updatedAt from Dart");
        assert_eq!(b.deleted_entries.len(), 1, "tombstone from Dart");
        assert!(!b.entries.iter().any(|e| e.id == b.deleted_entries[0].id));
    }

    fn fixture(name: &str) -> Book {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        Book::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn shape(b: &Book) -> (Vec<String>, Vec<(String, String, f64)>) {
        let mut ids: Vec<String> = b.entries.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        let mut acc: Vec<_> = b.accounts.iter().map(|a| (a.id.clone(), a.name.clone(), a.budget)).collect();
        acc.sort_by(|x, y| x.0.cmp(&y.0));
        (ids, acc)
    }

    /// Same fixtures and expectations as the Android tests (test/sync_test.dart).
    #[test]
    fn merge_matches_shared_expectation() {
        let (a, b) = (fixture("merge_a.json"), fixture("merge_b.json"));
        let exp: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/merge_expected.json")).unwrap();
        let (m, r) = a.merge(&b);
        let ids: Vec<&str> = exp["entryIds"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(shape(&m).0, ids);
        let names = exp["accounts"].as_object().unwrap();
        assert_eq!(m.accounts.len(), names.len());
        for (id, name) in names {
            assert_eq!(m.account(id).map(|a| a.name.as_str()), name.as_str(), "{id}");
        }
        assert_eq!(m.account("food").unwrap().budget, exp["foodBudget"].as_f64().unwrap());
        let rep = &exp["report"];
        let n = |k: &str| rep[k].as_u64().unwrap() as usize;
        assert_eq!(
            r,
            MergeReport {
                entries_added: n("entriesAdded"),
                entries_removed: n("entriesRemoved"),
                accounts_added: n("accountsAdded"),
                accounts_updated: n("accountsUpdated"),
                accounts_removed: n("accountsRemoved"),
                accounts_kept_in_use: n("accountsKeptInUse"),
                conflicts: n("conflicts"),
            }
        );
        // settings are per device
        assert_eq!((m.mode, m.theme), (Mode::Simple, ThemePref::Light));
        assert!(m.entries.iter().all(Entry::is_balanced));
    }

    #[test]
    fn merge_is_commutative_and_idempotent() {
        let (a, b) = (fixture("merge_a.json"), fixture("merge_b.json"));
        let (ab, _) = a.merge(&b);
        let (ba, _) = b.merge(&a);
        assert_eq!(shape(&ab), shape(&ba));
        assert!(ab.merge(&b).1.is_noop(), "{:?}", ab.merge(&b).1);
        assert!(ab.merge(&a).1.is_noop(), "{:?}", ab.merge(&a).1);
        assert!(ab.merge(&ab).1.is_noop());
        // exchanging the merged file both ways converges
        assert_eq!(shape(&ba.merge(&ab).0), shape(&ab));
    }

    #[test]
    fn timestamp_only_changes_are_not_reported_as_updates() {
        let a = demo();
        let mut b = a.clone();
        b.accounts[0].updated_at = 5; // same content, newer stamp
        let (m, r) = a.merge(&b);
        assert_eq!(r.accounts_updated, 0);
        assert_eq!(m.accounts[0].updated_at, 5);
    }

    #[test]
    fn deletion_travels_to_the_other_device() {
        let phone = demo();
        let mut pc = phone.clone();
        let victim = pc.entries[3].id.clone();
        pc.delete_entry(&victim);
        let id = pc
            .add_entry(
                &NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "", cr: "", amount: "99", date: "2026-11-20", memo: "" },
                new_entry_id(),
            )
            .unwrap();
        let (phone2, r) = phone.merge(&pc);
        assert_eq!((r.entries_added, r.entries_removed), (1, 1));
        assert!(phone2.entries.iter().any(|e| e.id == id));
        assert!(!phone2.entries.iter().any(|e| e.id == victim));
        // Undo on the pc brings it back everywhere, even though the phone
        // already knows about the deletion.
        let mut pc2 = pc.clone();
        let e = phone.entries.iter().find(|e| e.id == victim).unwrap().clone();
        let back = pc2.restore_entry(e.clone());
        let (pc3, _) = pc2.merge(&phone2);
        let (phone3, r) = phone2.merge(&pc3);
        assert_eq!(r.entries_added, 1);
        for b in [&pc3, &phone3] {
            let x = b.entries.iter().find(|x| x.id == back).expect("restored");
            assert_eq!((&x.date, &x.memo, &x.lines), (&e.date, &e.memo, &e.lines));
            assert_eq!(b.entries.iter().filter(|x| (&x.date, &x.memo, &x.lines) == (&e.date, &e.memo, &e.lines)).count(), 1);
        }
        assert!(pc3.merge(&phone3).1.is_noop() && phone3.merge(&pc3).1.is_noop());
    }

    #[test]
    fn starting_empty_removes_the_demo_and_nothing_else() {
        let mut b = demo();
        let n = b.entries.len();
        let f = NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "", cr: "", amount: "40", date: "2026-11-20", memo: "Mine" };
        b.add_entry(&f, new_entry_id()).unwrap();
        let (accounts, other) = (b.accounts.clone(), b.clone());
        assert_eq!((b.demo_count(), b.remove_demo()), (n, n));
        assert_eq!((b.demo_count(), b.entries.len(), b.accounts.clone()), (0, 1, accounts));
        assert_eq!(b.entries[0].memo, "Mine");
        // It holds against a device that has the same demo, and survives a save.
        assert_eq!(b.merge(&other).0.entries.len(), 1);
        assert_eq!(other.merge(&b).0.entries.len(), 1);
        let back = Book::from_json(&b.to_json()).unwrap();
        assert_eq!(back.entries, b.entries);
        assert_eq!(b.remove_demo(), 0);
        // A completely empty book saves, loads, and stays empty.
        let mut e = demo();
        e.remove_demo();
        let dir = tmpdir("empty");
        let path = dir.join("ledger.json");
        e.save(&path).unwrap();
        assert!(Book::load_or_seed(&path).unwrap().entries.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A French phone and an English computer, both fresh: same ids, other words.
    #[test]
    fn the_demo_in_two_languages_is_not_a_conflict() {
        let en = demo();
        let mut fr = en.clone();
        fr.accounts.iter_mut().for_each(|a| a.name.push_str(" (fr)"));
        fr.entries.iter_mut().for_each(|e| e.memo.push_str(" (fr)"));
        for (a, b) in [(&en, &fr), (&fr, &en)] {
            let (m, r) = a.merge(b);
            assert!(r.is_noop(), "{r:?}");
            assert_eq!((&m.accounts, &m.entries), (&a.accounts, &a.entries));
        }
        // An edit wins everywhere, and real conflicts are still counted.
        fr.save_account(Some("cash"), "Espèces", AccountType::Asset, "").unwrap();
        assert_eq!(en.merge(&fr).0.account("cash").unwrap().name, "Espèces");
        let (mut x, mut y) = (en.clone(), en.clone());
        for (b, name) in [(&mut x, "One"), (&mut y, "Two")] {
            b.accounts[0].updated_at = 5;
            b.accounts[0].name = name.into();
        }
        assert_eq!(x.merge(&y).1.conflicts, 1);
        let f = NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "", cr: "", amount: "40", date: "2026-11-20", memo: "One" };
        x = en.clone();
        x.add_entry(&f, "e1".into()).unwrap();
        y = en.clone();
        y.add_entry(&NewEntry { memo: "Two", ..f }, "e1".into()).unwrap();
        assert_eq!(x.merge(&y).1.conflicts, 1);
    }

    #[test]
    fn demo_entries_are_recognised() {
        assert!(demo().entries.iter().all(|e| is_demo_entry(&e.id)));
        for id in [new_entry_id().as_str(), "s", "s-", "s20261125-", "s2026112-1", "s20261125-x", "e20261125-1", "salary", "s2026é12-1"] {
            assert!(!is_demo_entry(id), "{id}");
        }
    }

    #[test]
    fn demo_seeds_from_different_days_do_not_share_ids() {
        let (x, y) = (Book::seed("2026-11-21"), Book::seed("2026-12-03"));
        assert!(x.entries.iter().all(|e| !y.entries.iter().any(|f| f.id == e.id)));
        assert_ne!(new_entry_id(), new_entry_id());
    }

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ledger-core-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn infinite_amounts_never_reach_the_file() {
        let mut b = demo();
        let f = NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "", cr: "", amount: "inf", date: "2026-11-20", memo: "" };
        for amount in ["inf", "1e400", "NaN"] {
            assert!(b.add_entry(&NewEntry { amount, ..f }, "x".into()).is_err(), "{amount}");
            let id = b.save_account(None, "Budget", AccountType::Expense, amount).unwrap();
            assert_eq!(b.account(&id).unwrap().budget, 0.0, "{amount}");
        }
        assert!(Book::from_json(&b.to_json()).is_ok());
    }

    /// Even if a bad number got into the book some other way, the good file on
    /// disk must not be replaced by one we could not read back.
    #[test]
    fn save_refuses_what_it_could_not_read_back() {
        let dir = tmpdir("save");
        let path = dir.join("ledger.json");
        let good = demo();
        good.save(&path).unwrap();
        let mut bad = good.clone();
        bad.entries[0].lines[0].dr = f64::INFINITY;
        assert!(bad.save(&path).is_err());
        let sync = folder::FolderSync::new(&dir, "desktop-aaaaaa");
        assert!(sync.write_own(&bad).is_err());
        assert!(!sync.own_path().exists());
        assert_eq!(Book::load_or_seed(&path).unwrap().entries, good.entries);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_file_is_set_aside_not_overwritten() {
        let dir = tmpdir("damaged");
        let path = dir.join("ledger.json");
        // A file written by the bug: an infinite amount saved as null.
        let broken = r#"{"accounts":[{"id":"cash","name":"Cash","type":"asset"}],"entries":[{"id":"e1","date":"2026-11-20","memo":"m","lines":[{"a":"cash","dr":null,"cr":0}]}]}"#;
        let mut copies = Vec::new();
        for _ in 0..2 {
            std::fs::write(&path, broken).unwrap();
            assert!(Book::load_or_seed(&path).is_err());
            let copy = set_aside(&path).unwrap();
            assert!(!path.exists());
            assert_eq!(std::fs::read_to_string(&copy).unwrap(), broken);
            copies.push(copy);
        }
        assert_ne!(copies[0], copies[1], "an earlier copy is never replaced");
        // The next launch reads what was saved since, instead of starting over.
        let mut b = demo();
        b.delete_entry(&b.entries[0].id.clone());
        b.save(&path).unwrap();
        assert_eq!(Book::load_or_seed(&path).unwrap().entries, b.entries);
        assert!(set_aside(&dir.join("missing.json")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn new_account_ids_are_unique() {
        let mut b = demo();
        let id = b.save_account(None, "Cash", AccountType::Asset, "").unwrap();
        assert_eq!(id, "cash-2");
    }
}
