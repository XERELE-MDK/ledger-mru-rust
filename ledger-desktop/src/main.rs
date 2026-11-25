//! Ledger MRU — desktop (Slint). Rust owns the state; the UI only renders what
//! `refresh()` pushes into the `Store` global and calls back on user actions.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use ledger_core::folder::{FolderSync, PeersStamp, new_device_id};
use ledger_core::*;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

slint::include_modules!();

struct State {
    book: Book,
    path: PathBuf,
    month: String,
    ledger_acc: String,
    // new-entry form (ids behind the combo boxes / chips)
    kind: Kind,
    cat: String,
    via_ids: Vec<String>,
    all_ids: Vec<String>,
    // open overlays
    open_entry: Option<String>,
    open_account: Option<String>,
    /// Opening balances form: (account id, text typed).
    opening: Vec<(String, String)>,
    undo: Option<Undo>,
    /// System preference, read once at start-up (before we override the palette).
    system_dark: bool,
    // ── folder sync (Syncthing)
    device_id: String,
    sync: Option<FolderSync>,
    /// Peer files already merged (change detection for polling).
    sync_stamp: Option<PeersStamp>,
    last_sync: Option<String>,
    peers: usize,
    /// Folder waiting for the user's "merge / use folder data" choice.
    pending_sync: Option<FolderSync>,
}

type Shared = Rc<RefCell<State>>;

/// What the toast's "Undo" puts back.
enum Undo {
    Entry(Entry),
    Book(Box<Book>),
    /// Also leaves the sync folder, or the next pass would merge it all again.
    Connect(Box<Book>),
}

/// "+3 entries, −1 entry, 1 account updated" (same wording as the Android app).
fn merge_summary(r: &MergeReport) -> String {
    let n = |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    let mut parts = Vec::new();
    if r.entries_added > 0 { parts.push(format!("+{}", n(r.entries_added, "entry", "entries"))) }
    if r.entries_removed > 0 { parts.push(format!("−{}", n(r.entries_removed, "entry", "entries"))) }
    if r.accounts_added > 0 { parts.push(format!("+{}", n(r.accounts_added, "account", "accounts"))) }
    if r.accounts_updated > 0 { parts.push(format!("{} updated", n(r.accounts_updated, "account", "accounts"))) }
    if r.accounts_removed > 0 { parts.push(format!("−{}", n(r.accounts_removed, "account", "accounts"))) }
    if r.accounts_kept_in_use > 0 {
        parts.push(format!("{} kept (still used)", n(r.accounts_kept_in_use, "account", "accounts")))
    }
    if r.conflicts > 0 { parts.push(format!("{} (kept this computer's)", n(r.conflicts, "conflict", "conflicts"))) }
    if parts.is_empty() { "nothing changed".into() } else { parts.join(", ") }
}

fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

fn ss(s: impl AsRef<str>) -> SharedString {
    SharedString::from(s.as_ref())
}

fn kind_index(k: Kind) -> i32 {
    match k {
        Kind::Expense => 0,
        Kind::Income => 1,
        Kind::Transfer => 2,
    }
}

fn entry_row(b: &Book, e: &Entry) -> EntryRow {
    let v = b.describe(e);
    let sign = match v.kind {
        Kind::Expense => "−",
        Kind::Income => "+",
        Kind::Transfer => "",
    };
    let mut lines: Vec<&Line> = e.lines.iter().collect();
    lines.sort_by(|x, y| y.dr.total_cmp(&x.dr));
    EntryRow {
        id: ss(&e.id),
        initial: ss(v.title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()),
        title: ss(&v.title),
        sub: ss(&v.sub),
        memo: ss(&e.memo),
        day: ss(day_short(&e.date)),
        day_long: ss(day_long(&e.date)),
        amount: ss(format!("{sign}{}", b.fmt(e.amount()))),
        kind: kind_index(v.kind),
        lines: model(
            lines
                .into_iter()
                .map(|l| LineRow {
                    name: ss(b.name_of(&l.a)),
                    dr: ss(if l.dr != 0.0 { b.fmt(l.dr) } else { String::new() }),
                    cr: ss(if l.cr != 0.0 { b.fmt(l.cr) } else { String::new() }),
                    credit: l.dr == 0.0,
                })
                .collect(),
        ),
    }
}

/// Recomputes every screen from the book and pushes it to the UI.
fn refresh(ui: &AppWindow, st: &State) {
    let s = ui.global::<Store>();
    let b = &st.book;
    let ym = st.month.as_str();
    let end = format!("{ym}-31");
    let m_entries = b.entries_in(ym);
    let t = b.totals(m_entries.iter().copied());
    let net = t.inc - t.exp;

    s.set_month_label(ss(ym_short(ym)));
    s.set_month_long(ss(ym_long(ym)));
    s.set_pro(b.mode == Mode::Pro);
    s.set_hints(b.hints);
    s.set_khoums(b.khoums);
    s.set_theme(match b.theme {
        ThemePref::System => 0,
        ThemePref::Light => 1,
        ThemePref::Dark => 2,
    });
    apply_theme(ui, st);

    // ── home
    s.set_kept_caps(ss(format!(
        "{} · {}",
        if net >= 0.0 { "YOU KEPT" } else { "YOU OVERSPENT" },
        ym_long(ym).to_uppercase()
    )));
    s.set_net(ss(f0(net.abs())));
    s.set_rate(ss(if t.inc > 0.0 {
        if net >= 0.0 {
            format!("{}% of your income stayed with you", (net / t.inc * 100.0).round())
        } else {
            format!("Spending was {}% of income", (t.exp / t.inc * 100.0).round())
        }
    } else {
        "No income recorded yet this month".into()
    }));
    s.set_inc(ss(f0(t.inc)));
    s.set_exp(ss(f0(t.exp)));
    let cats = b.spending_in(ym);
    let max_cat = cats.first().map_or(1.0, |c| c.2);
    s.set_cats(model(
        cats.iter()
            .take(6)
            .map(|(_, name, amt)| CatBar { name: ss(name), amount: ss(f0(*amt)), frac: (amt / max_cat) as f32 })
            .collect(),
    ));
    let assets: Vec<&Account> = b.sorted_accounts().into_iter().filter(|a| a.kind == AccountType::Asset).collect();
    s.set_assets_total(ss(f0(assets.iter().map(|a| b.raw_bal(&a.id, &end)).sum())));
    s.set_assets(model(
        assets.iter().map(|a| Pair { name: ss(&a.name), value: ss(f0(b.raw_bal(&a.id, &end))) }).collect(),
    ));
    s.set_recent(model(m_entries.iter().rev().take(6).map(|e| entry_row(b, e)).collect()));

    // ── journal
    let mut groups: Vec<(String, Vec<EntryRow>)> = Vec::new();
    for e in m_entries.iter().rev() {
        if groups.last().is_none_or(|g| g.0 != e.date) {
            groups.push((e.date.clone(), Vec::new()));
        }
        groups.last_mut().unwrap().1.push(entry_row(b, e));
    }
    s.set_groups(model(
        groups
            .into_iter()
            .map(|(d, items)| DayGroup { label: ss(day_long(&d).to_uppercase()), items: model(items) })
            .collect(),
    ));
    let (jdr, jcr) = m_entries
        .iter()
        .flat_map(|e| &e.lines)
        .fold((0.0, 0.0), |(d, c), l| (d + l.dr, c + l.cr));
    s.set_j_dr(ss(b.fmt(jdr)));
    s.set_j_cr(ss(b.fmt(jcr)));
    s.set_j_balanced((jdr - jcr).abs() < 0.01);

    // ── ledger
    let acc = b.sorted_accounts();
    s.set_ledger_accounts(model(acc.iter().map(|a| Opt { id: ss(&a.id), name: ss(&a.name) }).collect()));
    if let Some(la) = b.account(&st.ledger_acc).or(acc.first().copied()) {
        let mut run = b.raw_bal(&la.id, &format!("{ym}-00"));
        let open = run;
        let (mut tdr, mut tcr) = (0.0, 0.0);
        let mut rows = Vec::new();
        for e in &m_entries {
            for l in e.lines.iter().filter(|l| l.a == la.id) {
                run += l.dr - l.cr;
                tdr += l.dr;
                tcr += l.cr;
                rows.push(LedgerRow {
                    memo: ss(&e.memo),
                    date: ss(day_short(&e.date)),
                    dr: ss(if l.dr != 0.0 { b.fmt(l.dr) } else { String::new() }),
                    cr: ss(if l.cr != 0.0 { b.fmt(l.cr) } else { String::new() }),
                    bal: ss(b.fmt(Book::normal_bal(la, run))),
                });
            }
        }
        s.set_ledger_sel(ss(&la.id));
        s.set_l_name(ss(&la.name));
        s.set_l_type(ss(la.kind.label()));
        s.set_l_hint(ss(format!(
            "{} · normal side: {}",
            la.kind.hint(),
            if la.kind.debit_normal() { "debit" } else { "credit" }
        )));
        s.set_l_open(ss(b.fmt(Book::normal_bal(la, open))));
        s.set_l_close(ss(b.fmt(Book::normal_bal(la, run))));
        s.set_l_rows(model(rows));
        s.set_l_tot_dr(ss(b.fmt(tdr)));
        s.set_l_tot_cr(ss(b.fmt(tcr)));
    }

    // ── trial balance
    let (mut tdr, mut tcr) = (0.0, 0.0);
    let mut tb = Vec::new();
    for a in &acc {
        let r = b.raw_bal(&a.id, &end);
        if r.abs() < 0.005 {
            continue;
        }
        if r > 0.0 { tdr += r } else { tcr -= r }
        tb.push(TbRow {
            name: ss(&a.name),
            kind: ss(a.kind.label()),
            dr: ss(if r > 0.0 { b.fmt(r) } else { String::new() }),
            cr: ss(if r < 0.0 { b.fmt(-r) } else { String::new() }),
        });
    }
    s.set_tb_rows(model(tb));
    s.set_tb_dr(ss(b.fmt(tdr)));
    s.set_tb_cr(ss(b.fmt(tcr)));
    s.set_tb_balanced((tdr - tcr).abs() < 0.01);
    s.set_tb_asat(ss(format!("{} {}", day_short(&format!("{ym}-{:02}", days_in(ym))), &ym[..4])));

    // ── month over month
    let raw: Vec<(String, Totals)> = (0..6)
        .rev()
        .map(|i| {
            let m = ym_shift(ym, -i);
            let t = b.totals(b.entries_in(&m));
            (m, t)
        })
        .collect();
    let mx = raw.iter().fold(1.0_f64, |v, (_, t)| v.max(t.inc).max(t.exp));
    s.set_mom_rows(model(
        raw.iter()
            .map(|(m, t)| MonthRow {
                label: ss(month_abbr(m)),
                long: ss(ym_short(m)),
                inc: ss(f0(t.inc)),
                exp: ss(f0(t.exp)),
                net: ss(f0(t.inc - t.exp)),
                negative: t.inc - t.exp < 0.0,
                hi: (t.inc / mx) as f32,
                he: (t.exp / mx) as f32,
                current: m == ym,
            })
            .collect(),
    ));
    let with_data: Vec<&Totals> = raw.iter().map(|(_, t)| t).filter(|t| t.inc != 0.0 || t.exp != 0.0).collect();
    let prev = &raw[4].1;
    let compare = if prev.inc == 0.0 && prev.exp == 0.0 {
        "No data for last month yet.".to_string()
    } else {
        let d = t.exp - prev.exp;
        if d > 0.0 {
            format!("You spent {} MRU more than last month.", f0(d))
        } else if d < 0.0 {
            format!("You spent {} MRU less than last month.", f0(-d))
        } else {
            "Same spending as last month.".into()
        }
    };
    s.set_mom_summary(ss(if with_data.is_empty() {
        "No entries yet in the last six months.".to_string()
    } else {
        let avg = with_data.iter().map(|t| t.exp).sum::<f64>() / with_data.len() as f64;
        format!("Over the months with entries you spend about {} MRU a month. {compare}", f0(avg))
    }));

    // ── budgets
    let (mut btot, mut bsp) = (0.0, 0.0);
    let mut brows = Vec::new();
    for a in acc.iter().filter(|a| a.kind == AccountType::Expense && a.budget > 0.0) {
        let sp = cats.iter().find(|c| c.0 == a.id).map_or(0.0, |c| c.2);
        let p = sp / a.budget;
        btot += a.budget;
        bsp += sp;
        let level = if p > 1.0 { 2 } else if p >= 0.8 { 1 } else { 0 };
        brows.push(BudgetRow {
            id: ss(&a.id),
            name: ss(&a.name),
            spent: ss(f0(sp)),
            budget: ss(f0(a.budget)),
            frac: p as f32,
            level,
            status: ss(match level {
                2 => format!("{} MRU over budget", f0(sp - a.budget)),
                1 => format!("Close to limit · {} left", f0(a.budget - sp)),
                _ => format!("{} MRU left", f0(a.budget - sp)),
            }),
        });
    }
    s.set_bud_rows(model(brows));
    s.set_bud_spent(ss(f0(bsp)));
    s.set_bud_total(ss(f0(btot)));
    s.set_bud_frac(if btot > 0.0 { (bsp / btot) as f32 } else { 0.0 });

    // ── accounts
    s.set_acct_groups(model(
        AccountType::ORDER
            .iter()
            .filter(|t| acc.iter().any(|a| a.kind == **t))
            .map(|t| AcctGroup {
                label: ss(t.group_label().to_uppercase()),
                hint: ss(t.hint()),
                items: model(
                    acc.iter()
                        .filter(|a| a.kind == *t)
                        .map(|a| AcctRow {
                            id: ss(&a.id),
                            name: ss(&a.name),
                            meta: ss(if a.kind == AccountType::Expense && a.budget > 0.0 {
                                format!("Budget {} / month", f0(a.budget))
                            } else if matches!(a.kind, AccountType::Income | AccountType::Expense) {
                                "Total to date".into()
                            } else {
                                "Balance today".into()
                            }),
                            bal: ss(b.fmt(Book::normal_bal(a, b.raw_bal(&a.id, "9999-12-31")))),
                        })
                        .collect(),
                ),
            })
            .collect(),
    ));
    s.set_data_path(ss(st.path.display().to_string()));

    s.set_empty_book(b.entries.is_empty());

    // ── demo data still in the book
    let demo = b.demo_count();
    let own = b.entries.len() - demo;
    s.set_demo_count(demo as i32);
    s.set_only_demo(demo > 0 && own == 0);
    s.set_empty_label(ss(if own == 0 { "Start with an empty book" } else { "Remove the demo data" }));
    let n = |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    s.set_empty_text(ss(format!(
        "{} will be removed.{} Your accounts and budgets stay.{}",
        n(demo, "demo entry", "demo entries"),
        if own > 0 { format!(" Your {} stay.", n(own, "own entry", "own entries")) } else { String::new() },
        if st.sync.is_some() { " Sync is on: this applies to all your devices and cannot be undone." } else { "" }
    )));
}

/// Switches our palette (`P.dark`) and the std-widgets one (`Palette.color-scheme`).
fn apply_theme(ui: &AppWindow, st: &State) {
    use slint::language::ColorScheme;
    let (dark, scheme) = match st.book.theme {
        ThemePref::Dark => (true, ColorScheme::Dark),
        ThemePref::Light => (false, ColorScheme::Light),
        // Unknown = let the widgets follow the system themselves.
        ThemePref::System => (st.system_dark, ColorScheme::Unknown),
    };
    ui.global::<P>().set_dark(dark);
    if ui.global::<Palette>().get_color_scheme() != scheme {
        ui.global::<Palette>().set_color_scheme(scheme);
    }
}

/// Is the desktop in dark mode? Slint's own detection first, then the
/// freedesktop/GNOME setting most Linux desktops (Hyprland included) honour.
fn detect_system_dark(ui: &AppWindow) -> bool {
    use slint::language::ColorScheme;
    match ui.global::<Palette>().get_color_scheme() {
        ColorScheme::Dark => true,
        ColorScheme::Light => false,
        _ => std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("dark"))
            .unwrap_or(false),
    }
}

/// Category chips, combo-box options and the "will be recorded as" preview.
fn refresh_form(ui: &AppWindow, st: &mut State) {
    let s = ui.global::<Store>();
    let b = &st.book;
    s.set_f_kind(kind_index(st.kind));
    s.set_f_cat(ss(&st.cat));
    s.set_f_cats(model(
        b.category_options(st.kind).iter().map(|a| Opt { id: ss(&a.id), name: ss(&a.name) }).collect(),
    ));
    let via = b.via_options();
    st.via_ids = via.iter().map(|a| a.id.clone()).collect();
    s.set_via_names(model(via.iter().map(|a| ss(&a.name)).collect()));
    let all = b.sorted_accounts();
    st.all_ids = all.iter().map(|a| a.id.clone()).collect();
    s.set_all_names(model(all.iter().map(|a| ss(format!("{} · {}", a.name, a.kind.label()))).collect()));
    update_preview(ui, st);
}

fn pick(ids: &[String], i: i32) -> String {
    ids.get(i as usize).cloned().unwrap_or_default()
}

fn update_preview(ui: &AppWindow, st: &State) {
    let s = ui.global::<Store>();
    let (via, dr, cr) = (pick(&st.via_ids, s.get_via_index()), pick(&st.all_ids, s.get_dr_index()), pick(&st.all_ids, s.get_cr_index()));
    let amount = s.get_f_amount();
    let f = NewEntry { kind: st.kind, cat: &st.cat, via: &via, dr: &dr, cr: &cr, amount: &amount, date: "", memo: "" };
    let (d, c) = st.book.sides(&f);
    let name = |id: &str| if id.is_empty() { "—".to_string() } else { st.book.name_of(id) };
    s.set_p_dr(ss(name(d)));
    s.set_p_cr(ss(name(c)));
    s.set_p_amt(ss(st.book.fmt(parse_amount(&amount))));
}

fn persist(ui: &AppWindow, st: &State, toast: &Toast) {
    if let Err(e) = st.book.save(&st.path) {
        toast.show(ui, &format!("Could not save: {e}"), false);
    }
    // Keep this computer's file in the shared folder up to date.
    if let Some(fs) = &st.sync {
        if let Err(e) = fs.write_own(&st.book) {
            toast.show(ui, &format!("Sync folder: {e}"), false);
        }
    }
}

// ───────────────────────── folder sync (Syncthing) ─────────────────────────

/// `ledger.json` → `ledger.sync.conf`: this computer's id and chosen folder.
fn sync_conf_path(data: &Path) -> PathBuf {
    data.with_extension("sync.conf")
}

fn load_sync_conf(data: &Path) -> (String, Option<PathBuf>) {
    let text = std::fs::read_to_string(sync_conf_path(data)).unwrap_or_default();
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().to_string());
    let id = get("device=").filter(|v| !v.is_empty()).unwrap_or_else(|| new_device_id("desktop"));
    let folder = get("folder=").filter(|v| !v.is_empty()).map(PathBuf::from);
    save_sync_conf(data, &id, folder.as_deref());
    (id, folder)
}

fn save_sync_conf(data: &Path, id: &str, folder: Option<&Path>) {
    let _ = std::fs::create_dir_all(data.parent().unwrap_or(Path::new(".")));
    let body = format!("device={id}\nfolder={}\n", folder.map(|f| f.display().to_string()).unwrap_or_default());
    let _ = std::fs::write(sync_conf_path(data), body);
}

fn expand_home(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => dirs::home_dir().unwrap_or_default().join(rest),
        None => PathBuf::from(p),
    }
}

fn sync_status(ui: &AppWindow, st: &State) {
    let s = ui.global::<Store>();
    s.set_sync_on(st.sync.is_some());
    s.set_sync_device(ss(&st.device_id));
    if let Some(fs) = &st.sync {
        s.set_sync_path(ss(fs.dir.display().to_string()));
        let others = match st.peers {
            0 => "no other device yet".to_string(),
            1 => "1 other device".to_string(),
            n => format!("{n} other devices"),
        };
        s.set_sync_status(ss(match &st.last_sync {
            Some(t) => format!("Synced at {t} · {others}"),
            None => format!("Connected · {others}"),
        }));
    }
}

/// One sync pass: merge peers, save, rewrite our file, refresh.
fn run_sync(ui: &AppWindow, st: &mut State, toast: &Toast, quiet: bool) {
    let Some(fs) = st.sync.clone() else { return };
    let mut book = st.book.clone();
    // Taken before reading: a file that changes meanwhile is seen next time.
    let stamp = fs.peers_stamp();
    match fs.sync(&mut book) {
        Ok(o) => {
            st.peers = o.peers.len();
            st.last_sync = Some(now_hhmm());
            st.sync_stamp = stamp;
            let changed = !o.report.is_noop();
            if changed {
                st.book = book;
                if let Err(e) = st.book.save(&st.path) {
                    toast.show(ui, &format!("Could not save: {e}"), false);
                }
                refresh(ui, st);
            }
            if changed {
                toast.show(ui, &format!("Synced: {}", merge_summary(&o.report)), false);
            } else if !o.skipped.is_empty() {
                toast.show(ui, &format!("Skipped unreadable file {}", o.skipped[0].0), false);
            } else if !quiet {
                toast.show(ui, "Up to date", false);
            }
        }
        Err(e) => {
            if !quiet {
                toast.show(ui, &format!("Sync failed: {e}"), false);
            }
        }
    }
    sync_status(ui, st);
}

fn connect(ui: &AppWindow, st: &mut State, fs: FolderSync) {
    save_sync_conf(&st.path, &st.device_id, Some(&fs.dir));
    st.sync = Some(fs);
    st.last_sync = None;
    st.sync_stamp = None;
    ui.global::<Store>().set_sync_choice(false);
}

/// Bottom-centre message that hides itself.
struct Toast(Timer);

impl Toast {
    fn show(&self, ui: &AppWindow, msg: &str, undo: bool) {
        let s = ui.global::<Store>();
        s.set_toast(ss(msg));
        s.set_toast_undo(undo);
        let w = ui.as_weak();
        self.0.start(TimerMode::SingleShot, Duration::from_millis(if undo { 5000 } else { 2200 }), move || {
            if let Some(ui) = w.upgrade() {
                ui.global::<Store>().set_toast(SharedString::new());
            }
        });
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--screenshots") {
        let dir = PathBuf::from(args.get(i + 1).map_or("screenshots", |s| s.as_str()));
        return shots::run(&dir);
    }
    let path = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("ledger-mru").join("ledger.json");
    let (ui, _state) = app(path)?;
    // Lets Wayland/X11 match the window to packaging/app.mru.ledger.desktop,
    // which carries the icon. Must happen before the window is shown.
    slint::set_xdg_app_id("app.mru.ledger")?;
    ui.run()
}

// Entries would disappear here without a trace, and the other devices would
// send them back; the demo would go the other way.
const SYNC_FIRST_RESET: &str = "Sync is on: the demo data would be sent to your other devices, and your entries would come back from them. Disconnect the sync folder first.";
const SYNC_FIRST_REPLACE: &str = "Sync is on: your other devices would send back the entries you are replacing. Use Merge, or disconnect the sync folder first.";

/// Why the demo data is showing instead of the user's ledger.
enum Trouble {
    /// The file's content is unreadable: kept, untouched, at this path.
    SetAside(PathBuf),
    /// The file itself cannot be opened (reason): left alone, saving elsewhere.
    Locked(String, PathBuf),
}

impl Trouble {
    fn text(&self) -> String {
        match self {
            Trouble::SetAside(copy) => format!(
                "Your ledger could not be read, so demo data is shown. Nothing was deleted: your file is kept as {}. \
                 Sync was turned off; if you use it, connect the folder again and choose \u{201c}Use folder data\u{201d}.",
                copy.display()
            ),
            Trouble::Locked(why, path) => format!(
                "{why}. Demo data is shown and changes are saved to {}. Your file was not touched; sync is off until it can be read again.",
                path.display()
            ),
        }
    }
}

/// The book at `path`, where to save it, and what went wrong if it did not load.
/// A file we failed to read is never overwritten.
fn load(path: PathBuf) -> (Book, PathBuf, Option<Trouble>) {
    let err = match Book::load_or_seed(&path) {
        Ok(b) => return (b, path, None),
        Err(e) => e,
    };
    let demo = Book::seed(&today());
    match set_aside(&path) {
        Ok(copy) => (demo, path, Some(Trouble::SetAside(copy))),
        Err(_) => {
            // A new name each time: never replace an earlier recovery file.
            let alt = path.with_file_name(format!("ledger.recovered-{}.json", now_ms()));
            (demo, alt.clone(), Some(Trouble::Locked(err, alt)))
        }
    }
}

/// Builds the window, loads the book from `path` and wires every callback.
fn app(path: PathBuf) -> Result<(AppWindow, Shared), slint::PlatformError> {
    let (book, path, trouble) = load(path);
    let st: Shared = Rc::new(RefCell::new(State {
        book,
        path,
        month: today()[..7].to_string(),
        ledger_acc: "cash".into(),
        kind: Kind::Expense,
        cat: String::new(),
        via_ids: vec![],
        all_ids: vec![],
        open_entry: None,
        open_account: None,
        opening: vec![],
        undo: None,
        system_dark: false,
        device_id: String::new(),
        sync: None,
        sync_stamp: None,
        last_sync: None,
        peers: 0,
        pending_sync: None,
    }));
    {
        let mut s = st.borrow_mut();
        if let Some(Trouble::SetAside(_)) = &trouble {
            // The demo must never reach the shared folder, nor replace the copy
            // of the real data this computer left there. With a new device id,
            // that copy becomes another device's file: reconnecting offers it.
            save_sync_conf(&s.path, &new_device_id("desktop"), None);
        }
        let (id, folder) = load_sync_conf(&s.path);
        s.device_id = id.clone();
        s.sync = folder.map(|f| FolderSync::new(f, id));
    }

    let ui = AppWindow::new()?;
    st.borrow_mut().system_dark = detect_system_dark(&ui);
    let toast = Rc::new(Toast(Timer::default()));
    let store = ui.global::<Store>();
    let export_path = dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_default()
        .join("ledger-mru-export.json");
    store.set_io_path(ss(export_path.display().to_string()));
    store.set_sync_path(ss(dirs::home_dir().unwrap_or_default().join("Sync").join("Ledger").display().to_string()));
    refresh(&ui, &st.borrow());
    sync_status(&ui, &st.borrow());
    if let Some(t) = trouble {
        store.set_notice(ss(t.text()));
    }

    // Small helper to wire a callback with access to ui + state + toast.
    macro_rules! on {
        ($name:ident, |$ui:ident, $st:ident, $toast:ident $(, $arg:ident : $ty:ty)*| $body:block) => {{
            let w = ui.as_weak();
            let st = st.clone();
            let toast = toast.clone();
            store.$name(move |$($arg: $ty),*| {
                let $ui = w.unwrap();
                #[allow(unused_mut)]
                let mut $st = st.borrow_mut();
                #[allow(unused_variables)]
                let $toast = &*toast;
                $body
            });
        }};
    }

    on!(on_prev_month, |ui, st, _t| {
        st.month = ym_shift(&st.month, -1);
        refresh(&ui, &st);
    });
    on!(on_next_month, |ui, st, _t| {
        st.month = ym_shift(&st.month, 1);
        refresh(&ui, &st);
    });
    on!(on_this_month, |ui, st, _t| {
        st.month = today()[..7].to_string();
        refresh(&ui, &st);
    });
    on!(on_pick_ledger, |ui, st, _t, id: SharedString| {
        st.ledger_acc = id.to_string();
        refresh(&ui, &st);
    });
    on!(on_set_mode, |ui, st, t, pro: bool| {
        st.book.mode = if pro { Mode::Pro } else { Mode::Simple };
        persist(&ui, &st, t);
        refresh(&ui, &st);
    });
    on!(on_set_hints, |ui, st, t, v: bool| {
        st.book.hints = v;
        persist(&ui, &st, t);
        refresh(&ui, &st);
    });
    on!(on_set_theme, |ui, st, t, i: i32| {
        st.book.theme = [ThemePref::System, ThemePref::Light, ThemePref::Dark][i.clamp(0, 2) as usize];
        persist(&ui, &st, t);
        refresh(&ui, &st);
    });
    on!(on_set_khoums, |ui, st, t, v: bool| {
        st.book.khoums = v;
        persist(&ui, &st, t);
        refresh(&ui, &st);
    });

    // ── new entry
    on!(on_open_add, |ui, st, _t| {
        let s = ui.global::<Store>();
        st.kind = Kind::Expense;
        st.cat = st.book.category_options(Kind::Expense).first().map(|a| a.id.clone()).unwrap_or_default();
        refresh_form(&ui, &mut st);
        let first_exp = st.all_ids.iter().position(|id| st.book.type_of(id) == Some(AccountType::Expense));
        let first_via = st.via_ids.first().and_then(|v| st.all_ids.iter().position(|id| id == v));
        s.set_via_index(0);
        s.set_dr_index(first_exp.unwrap_or(0) as i32);
        s.set_cr_index(first_via.unwrap_or(0) as i32);
        let t = today();
        s.set_f_date(ss(if t[..7] == st.month { t } else { format!("{}-01", st.month) }));
        s.set_f_amount(SharedString::new());
        s.set_f_memo(SharedString::new());
        s.set_f_error(SharedString::new());
        update_preview(&ui, &st);
        s.set_overlay(1);
    });
    on!(on_set_kind, |ui, st, _t, k: i32| {
        st.kind = [Kind::Expense, Kind::Income, Kind::Transfer][k.clamp(0, 2) as usize];
        let via = pick(&st.via_ids, ui.global::<Store>().get_via_index());
        let opts = st.book.category_options(st.kind);
        let c = if st.kind == Kind::Transfer { opts.iter().find(|o| o.id != via).or(opts.first()) } else { opts.first() };
        st.cat = c.map(|a| a.id.clone()).unwrap_or_default();
        ui.global::<Store>().set_f_error(SharedString::new());
        refresh_form(&ui, &mut st);
    });
    on!(on_pick_cat, |ui, st, _t, id: SharedString| {
        st.cat = id.to_string();
        ui.global::<Store>().set_f_cat(id);
        ui.global::<Store>().set_f_error(SharedString::new());
        update_preview(&ui, &st);
    });
    on!(on_form_edited, |ui, st, _t| {
        ui.global::<Store>().set_f_error(SharedString::new());
        update_preview(&ui, &st);
    });
    on!(on_save_entry, |ui, st, t| {
        let s = ui.global::<Store>();
        let (via, dr, cr) = (pick(&st.via_ids, s.get_via_index()), pick(&st.all_ids, s.get_dr_index()), pick(&st.all_ids, s.get_cr_index()));
        let (amount, date, memo) = (s.get_f_amount(), s.get_f_date(), s.get_f_memo());
        let kind = st.kind;
        let cat = st.cat.clone();
        let f = NewEntry { kind, cat: &cat, via: &via, dr: &dr, cr: &cr, amount: &amount, date: date.trim(), memo: &memo };
        let id = new_entry_id();
        match st.book.add_entry(&f, id) {
            Ok(_) => {
                st.month = date.trim()[..7].to_string();
                persist(&ui, &st, t);
                s.set_overlay(0);
                refresh(&ui, &st);
                t.show(&ui, "Entry saved", false);
            }
            Err(e) => s.set_f_error(ss(e)),
        }
    });
    on!(on_close_overlay, |ui, st, _t| {
        st.open_entry = None;
        st.open_account = None;
        ui.global::<Store>().set_overlay(0);
    });

    // ── entry detail
    on!(on_open_entry, |ui, st, _t, id: SharedString| {
        if let Some(e) = st.book.entries.iter().find(|e| e.id == id.as_str()) {
            ui.global::<Store>().set_cur(entry_row(&st.book, e));
            st.open_entry = Some(id.to_string());
            ui.global::<Store>().set_overlay(2);
        }
    });
    on!(on_delete_entry, |ui, st, t| {
        if let Some(id) = st.open_entry.take() {
            st.undo = st.book.delete_entry(&id).map(Undo::Entry);
            persist(&ui, &st, t);
            ui.global::<Store>().set_overlay(0);
            refresh(&ui, &st);
            t.show(&ui, "Entry deleted", true);
        }
    });
    on!(on_undo, |ui, st, t| {
        match st.undo.take() {
            Some(Undo::Entry(e)) => {
                st.book.restore_entry(e);
                persist(&ui, &st, t);
                refresh(&ui, &st);
                t.show(&ui, "Entry restored", false);
            }
            Some(Undo::Book(b)) => {
                st.book = *b;
                persist(&ui, &st, t);
                refresh(&ui, &st);
                ui.global::<Store>().set_notice(SharedString::new());
                t.show(&ui, "Undone", false);
            }
            Some(Undo::Connect(b)) => {
                st.sync = None;
                st.last_sync = None;
                save_sync_conf(&st.path, &st.device_id, None);
                st.book = *b;
                persist(&ui, &st, t);
                refresh(&ui, &st);
                sync_status(&ui, &st);
                t.show(&ui, "Undone, sync is off", false);
            }
            None => {}
        }
    });

    // ── account editor
    on!(on_open_account, |ui, st, _t, id: SharedString| {
        let s = ui.global::<Store>();
        s.set_af_error(SharedString::new());
        match st.book.account(&id).cloned() {
            Some(a) => {
                s.set_af_editing(true);
                s.set_af_name(ss(&a.name));
                s.set_af_type(a.kind.rank() as i32);
                s.set_af_budget(ss(if a.budget > 0.0 { f2(a.budget, false).replace(',', "") } else { String::new() }));
                st.open_account = Some(a.id);
            }
            None => {
                s.set_af_editing(false);
                s.set_af_name(SharedString::new());
                s.set_af_type(4);
                s.set_af_budget(SharedString::new());
                st.open_account = None;
            }
        }
        s.set_overlay(3);
    });
    on!(on_save_account, |ui, st, t| {
        let s = ui.global::<Store>();
        let kind = AccountType::ORDER[s.get_af_type().clamp(0, 4) as usize];
        let id = st.open_account.clone();
        match st.book.save_account(id.as_deref(), &s.get_af_name(), kind, &s.get_af_budget()) {
            Ok(_) => {
                persist(&ui, &st, t);
                s.set_overlay(0);
                refresh(&ui, &st);
                t.show(&ui, "Account saved", false);
            }
            Err(e) => s.set_af_error(ss(e)),
        }
    });
    on!(on_delete_account, |ui, st, t| {
        let s = ui.global::<Store>();
        let Some(id) = st.open_account.clone() else { return };
        match st.book.delete_account(&id) {
            Ok(()) => {
                if st.ledger_acc == id {
                    st.ledger_acc = "cash".into();
                }
                persist(&ui, &st, t);
                s.set_overlay(0);
                refresh(&ui, &st);
                t.show(&ui, "Account deleted", false);
            }
            Err(e) => s.set_af_error(ss(e)),
        }
    });

    // ── opening balances
    on!(on_open_opening, |ui, st, _t| {
        let s = ui.global::<Store>();
        let plain = |v: f64| if v == 0.0 { String::new() } else { f2(v, false).replace(',', "") };
        let rows: Vec<(String, String, bool, String)> =
            st.book.opening_rows().into_iter().map(|(a, v)| (a.id.clone(), a.name.clone(), !a.kind.debit_normal(), plain(v))).collect();
        s.set_open_rows(model(rows.iter().map(|(_, name, debt, text)| OpenRow { name: ss(name), debt: *debt, amount: ss(text) }).collect()));
        s.set_open_date(ss(st.book.opening_date().map_or_else(today, str::to_string)));
        s.set_open_error(SharedString::new());
        st.opening = rows.into_iter().map(|(id, _, _, text)| (id, text)).collect();
        s.set_overlay(4);
    });
    on!(on_edit_opening, |ui, st, _t, i: i32, text: SharedString| {
        if let Some(r) = st.opening.get_mut(i as usize) {
            r.1 = text.to_string();
        }
        ui.global::<Store>().set_open_error(SharedString::new());
    });
    on!(on_save_opening, |ui, st, t| {
        let s = ui.global::<Store>();
        let date = s.get_open_date().trim().to_string();
        let amounts = st.opening.clone();
        match st.book.set_opening(&date, &amounts, "Opening balances") {
            Ok(id) => {
                if id.is_some() {
                    st.month = date[..7].to_string();
                }
                persist(&ui, &st, t);
                s.set_overlay(0);
                refresh(&ui, &st);
                t.show(&ui, if id.is_some() { "Opening balances saved" } else { "Opening balances removed" }, false);
            }
            Err(e) => s.set_open_error(ss(e)),
        }
    });

    // ── data
    on!(on_reset_demo, |ui, st, t| {
        ui.global::<Store>().set_confirm_reset(false);
        if st.sync.is_some() {
            ui.global::<Store>().set_notice(ss(SYNC_FIRST_RESET));
            return;
        }
        st.book = Book::seed(&today());
        st.ledger_acc = "cash".into();
        persist(&ui, &st, t);
        refresh(&ui, &st);
        t.show(&ui, "Demo data restored", false);
    });
    on!(on_start_empty, |ui, st, t| {
        let s = ui.global::<Store>();
        s.set_confirm_empty(false);
        let before = st.book.clone();
        let n = st.book.remove_demo();
        if n == 0 {
            return;
        }
        // Undone here, the other devices would remove them again.
        let undo = st.sync.is_none();
        st.undo = undo.then(|| Undo::Book(Box::new(before)));
        persist(&ui, &st, t);
        refresh(&ui, &st);
        t.show(&ui, "Demo data removed", undo);
    });
    on!(on_export_json, |ui, st, t| {
        let p = PathBuf::from(ui.global::<Store>().get_io_path().trim());
        match std::fs::write(&p, st.book.to_json()) {
            Ok(()) => t.show(&ui, &format!("Exported {} entries", st.book.entries.len()), false),
            Err(e) => t.show(&ui, &format!("Export failed: {e}"), false),
        }
    });
    on!(on_import_json, |ui, st, t, merge: bool| {
        if !merge && st.sync.is_some() {
            ui.global::<Store>().set_notice(ss(SYNC_FIRST_REPLACE));
            return;
        }
        let p = PathBuf::from(ui.global::<Store>().get_io_path().trim());
        match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Book::from_json(&s)) {
            Ok(file) => {
                let before = st.book.clone();
                let msg = if merge {
                    let (merged, r) = st.book.merge(&file);
                    if r.is_noop() {
                        t.show(&ui, "Already up to date: nothing new in this file", false);
                        return;
                    }
                    st.book = merged;
                    format!("Merged: {}", merge_summary(&r))
                } else {
                    st.book = file;
                    format!("Replaced with {} entries", st.book.entries.len())
                };
                st.undo = Some(Undo::Book(Box::new(before)));
                persist(&ui, &st, t);
                refresh(&ui, &st);
                t.show(&ui, &msg, true);
            }
            Err(e) => t.show(&ui, &format!("Import failed: {e}"), false),
        }
    });

    // ── folder sync
    on!(on_sync_connect, |ui, st, t| {
        let s = ui.global::<Store>();
        let dir = expand_home(s.get_sync_path().trim());
        if let Err(e) = std::fs::create_dir_all(&dir) {
            t.show(&ui, &format!("Cannot use this folder: {e}"), false);
            return;
        }
        let fs = FolderSync::new(&dir, st.device_id.clone());
        let (joining, demo) = match fs.without_local_demo(&st.book) {
            Ok(x) => x,
            Err(e) => {
                t.show(&ui, &e, false);
                return;
            }
        };
        match fs.preview(&joining) {
            Ok(p) if p.peers.is_empty() || (p.report.is_noop() && demo == 0) => {
                connect(&ui, &mut st, fs);
                run_sync(&ui, &mut st, t, true);
                t.show(&ui, "Connected: this computer's file is now in the folder", false);
            }
            Ok(p) => {
                s.set_sync_choice_text(ss(format!(
                    "This folder already holds data from {}. Merging changes this computer: {}.{}",
                    if p.peers.len() == 1 { "1 other device".to_string() } else { format!("{} other devices", p.peers.len()) },
                    merge_summary(&p.report),
                    match demo {
                        0 => String::new(),
                        1 => " The demo entry here is removed, so that it does not reach your other devices.".into(),
                        n => format!(" The {n} demo entries here are removed, so that they do not reach your other devices."),
                    }
                )));
                st.pending_sync = Some(fs);
                s.set_sync_choice(true);
            }
            Err(e) => t.show(&ui, &e, false),
        }
    });
    on!(on_sync_choose, |ui, st, t, choice: i32| {
        let Some(fs) = st.pending_sync.take() else { return };
        ui.global::<Store>().set_sync_choice(false);
        if choice == 0 {
            return;
        }
        let before = st.book.clone();
        if choice == 2 {
            // Use the folder's data: start from the peers only, keep local settings.
            let peers: Vec<Book> = fs.peers().unwrap_or_default().into_iter().filter_map(|p| p.book.ok()).collect();
            if let Some((first, rest)) = peers.split_first() {
                let mut acc = first.clone();
                for b in rest {
                    acc = acc.merge(b).0;
                }
                acc.mode = st.book.mode;
                acc.hints = st.book.hints;
                acc.khoums = st.book.khoums;
                acc.theme = st.book.theme;
                st.book = acc;
            }
        } else {
            match fs.without_local_demo(&st.book) {
                Ok((joining, _)) => st.book = joining,
                Err(e) => {
                    t.show(&ui, &e, false);
                    return;
                }
            }
        }
        connect(&ui, &mut st, fs);
        run_sync(&ui, &mut st, t, true);
        persist(&ui, &st, t);
        refresh(&ui, &st);
        st.undo = Some(Undo::Connect(Box::new(before)));
        t.show(&ui, if choice == 2 { "Connected: using the folder's data" } else { "Connected and merged" }, true);
    });
    on!(on_sync_now, |ui, st, t| {
        run_sync(&ui, &mut st, t, false);
    });
    on!(on_sync_disconnect, |ui, st, t| {
        st.sync = None;
        st.last_sync = None;
        save_sync_conf(&st.path, &st.device_id, None);
        sync_status(&ui, &st);
        t.show(&ui, "Sync stopped (this computer's file stays in the folder)", false);
    });

    // Watch the folder: Syncthing drops the phone's file there whenever it changes.
    let poll = Timer::default();
    {
        let w = ui.as_weak();
        let st = st.clone();
        let toast = toast.clone();
        poll.start(TimerMode::Repeated, Duration::from_secs(4), move || {
            let Some(ui) = w.upgrade() else { return };
            let Ok(mut st) = st.try_borrow_mut() else { return };
            let Some(fs) = &st.sync else { return };
            if fs.peers_stamp() != st.sync_stamp {
                run_sync(&ui, &mut st, &toast, true);
            }
        });
    }
    if st.borrow().sync.is_some() {
        run_sync(&ui, &mut st.borrow_mut(), &toast, true);
    }
    std::mem::forget(poll); // lives as long as the window

    Ok((ui, st))
}


/// `--screenshots <dir>`: renders every screen offscreen with the software
/// renderer (no window opened) on demo data in a temp file, and writes PPM images.
mod shots {
    use super::*;
    use slint::platform::software_renderer::{MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType};
    use slint::platform::{Platform, WindowAdapter};
    use slint::PhysicalSize;

    struct Offscreen(Rc<MinimalSoftwareWindow>);

    impl Platform for Offscreen {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    pub fn run(dir: &std::path::Path) -> Result<(), slint::PlatformError> {
        let (w, h) = (1240u32, 800u32);
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Offscreen(window.clone())))
            .map_err(|e| slint::PlatformError::Other(e.to_string()))?;
        let data = std::env::temp_dir().join(format!("ledger-shots-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&data);
        let (ui, st) = app(data.clone())?;
        window.set_size(PhysicalSize::new(w, h));
        ui.show()?;
        std::fs::create_dir_all(dir).map_err(|e| slint::PlatformError::Other(e.to_string()))?;

        let snap = |name: &str| {
            slint::platform::update_timers_and_animations();
            window.request_redraw();
            let sz = window.size();
            let (w, h) = (sz.width, sz.height);
            window.draw_if_needed(|r| {
                let mut buf = vec![PremultipliedRgbaColor::default(); (w * h) as usize];
                r.render(&mut buf, w as usize);
                let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
                for p in &buf {
                    out.extend([p.red, p.green, p.blue]);
                }
                std::fs::write(dir.join(format!("{name}.ppm")), out).expect("write screenshot");
            });
        };

        let s = ui.global::<Store>();
        s.invoke_set_theme(1); // light first, whatever the desktop prefers
        snap("0_first_launch");
        // Start empty: every screen must cope with no entry at all; then undo.
        let demo = st.borrow().book.entries.len();
        assert!(s.get_only_demo());
        s.invoke_start_empty();
        assert!(st.borrow().book.entries.is_empty() && !s.get_only_demo() && s.get_demo_count() == 0);
        for tab in 0..5 {
            s.set_tab(tab);
            for report in 0..3 {
                s.set_report(report);
                snap(&format!("0_empty_{tab}_{report}"));
            }
        }
        s.invoke_undo();
        assert_eq!(st.borrow().book.entries.len(), demo);
        s.invoke_start_empty();
        // Opening balances: an error, then a real one, changed, then removed.
        s.set_tab(0);
        s.invoke_open_opening();
        s.invoke_save_opening();
        assert!(!s.get_open_error().is_empty());
        snap("0_opening_error");
        s.invoke_edit_opening(0, "3,000".into());
        s.invoke_edit_opening(4, "500".into());
        s.invoke_save_opening();
        assert_eq!(s.get_assets_total().as_str(), "3,000");
        assert!(!s.get_empty_book());
        snap("0_opening_saved");
        s.invoke_open_opening();
        assert_eq!(slint::Model::row_data(&s.get_open_rows(), 0).unwrap().amount.as_str(), "3000");
        snap("0_opening_form");
        s.invoke_edit_opening(0, "0".into());
        s.invoke_edit_opening(4, "".into());
        s.invoke_save_opening();
        assert!(s.get_empty_book());
        st.borrow_mut().undo = None;
        s.invoke_reset_demo();
        assert_eq!(st.borrow().book.entries.len(), demo);
        s.set_tab(0);
        s.set_report(0);
        s.set_demo_seen(true);
        snap("1_home");
        s.set_tab(1);
        snap("2_journal");
        s.set_tab(2);
        snap("3_ledger");
        s.set_tab(3);
        snap("4_trial_balance");
        s.set_report(1);
        snap("5_monthly");
        s.set_report(2);
        snap("6_budgets");
        s.set_tab(4);
        snap("7_accounts");
        s.invoke_set_mode(true);
        s.set_tab(1);
        snap("8_journal_pro");
        s.invoke_set_mode(false);
        s.set_tab(0);
        s.invoke_open_add();
        snap("9_new_entry");
        s.set_f_amount("abc".into());
        s.invoke_save_entry();
        snap("10_new_entry_error");
        s.set_f_amount("450".into());
        s.invoke_save_entry();
        snap("11_saved_toast");
        let first = s.get_recent();
        if let Some(e) = slint::Model::row_data(&first, 0) {
            s.invoke_open_entry(e.id);
            snap("12_entry_detail");
        }
        s.invoke_close_overlay();
        s.invoke_set_theme(2);
        s.set_tab(0);
        snap("13_dark_home");
        s.set_tab(1);
        snap("14_dark_journal");
        s.set_tab(3);
        s.set_report(1);
        snap("15_dark_monthly");
        s.set_tab(4);
        snap("16_dark_accounts");
        s.invoke_open_add();
        snap("17_dark_new_entry");
        // End-to-end merge through the UI callbacks, with the shared fixture.
        s.invoke_close_overlay();
        s.invoke_set_theme(1);
        let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../ledger-core/tests/fixtures/merge_b.json");
        s.set_io_path(fixture.into());
        s.set_tab(4);
        s.invoke_import_json(true);
        snap("18_merged_toast");
        s.set_tab(0);
        snap("19_merged_home");
        s.invoke_undo();
        snap("20_merge_undone");

        // Folder sync: a temp folder that already holds a phone's file.
        let sdir = std::env::temp_dir().join(format!("ledger-shots-sync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&sdir);
        std::fs::create_dir_all(&sdir).unwrap();
        let phone_file = sdir.join("ledger-android-test01.json");
        std::fs::copy(fixture, &phone_file).unwrap();
        s.set_sync_path(sdir.display().to_string().into());
        window.set_size(PhysicalSize::new(w, 1500)); // tall enough to show the Sync card
        s.set_tab(4);
        s.invoke_sync_connect();
        snap("21_sync_choice");
        s.invoke_sync_choose(1);
        snap("22_sync_on");
        let own = std::fs::read_dir(&sdir).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("ledger-desktop-")).count();
        assert_eq!(own, 1, "desktop file written to the sync folder");
        let own_book = || {
            let f = std::fs::read_dir(&sdir).unwrap().flatten().find(|e| e.file_name().to_string_lossy().starts_with("ledger-desktop-")).unwrap();
            Book::from_json(&std::fs::read_to_string(f.path()).unwrap()).unwrap()
        };
        assert!(own_book().entries.iter().all(|e| !is_demo_entry(&e.id)), "the demo stays out of the folder");
        // Reset and Replace are refused while sync is on, with an explanation.
        let count = own_book().entries.len();
        s.invoke_reset_demo();
        assert_eq!(s.get_notice().as_str(), SYNC_FIRST_RESET);
        s.invoke_import_json(false);
        assert_eq!(s.get_notice().as_str(), SYNC_FIRST_REPLACE);
        snap("22b_sync_first");
        s.set_notice(SharedString::new());
        assert_eq!(own_book().entries.len(), count);
        // Undoing a deletion the other device already knows about.
        let victim = own_book().entries.last().unwrap().clone();
        s.invoke_open_entry(ss(&victim.id));
        s.invoke_delete_entry();
        std::fs::write(&phone_file, own_book().to_json()).unwrap(); // the phone has merged the deletion
        s.invoke_undo();
        s.invoke_sync_now();
        s.invoke_sync_now();
        let back = own_book();
        assert!(back.entries.iter().any(|e| e.memo == victim.memo && e.date == victim.date && e.lines == victim.lines), "undo survives sync");
        assert!(!back.entries.iter().any(|e| e.id == victim.id));
        // The phone adds an entry; Syncthing drops the new file in the folder.
        let mut phone = Book::from_json(&std::fs::read_to_string(&phone_file).unwrap()).unwrap();
        let f = NewEntry { kind: Kind::Expense, cat: "food", via: "cash", dr: "food", cr: "cash", amount: "777", date: &today(), memo: "From the phone" };
        phone.add_entry(&f, new_entry_id()).unwrap();
        std::fs::write(&phone_file, phone.to_json()).unwrap();
        s.invoke_sync_now();
        snap("23_synced_from_phone");
        s.set_tab(0);
        snap("24_home_after_sync");
        s.set_notice(ss(Trouble::SetAside("/home/me/.local/share/ledger-mru/ledger.damaged-20261125-174500.json".into()).text()));
        window.set_size(PhysicalSize::new(1000, 640)); // smallest window
        snap("25_notice");
        s.set_notice(SharedString::new());
        s.invoke_sync_disconnect();
        // Undoing a connection puts the book back and leaves the folder.
        let before = st.borrow().book.entries.len();
        std::fs::write(&phone_file, include_str!("../../ledger-core/tests/fixtures/merge_a.json")).unwrap();
        s.invoke_sync_connect();
        if s.get_sync_choice() {
            s.invoke_sync_choose(1);
        }
        assert!(s.get_sync_on());
        s.invoke_undo();
        assert!(!s.get_sync_on());
        assert_eq!(st.borrow().book.entries.len(), before);
        let _ = std::fs::remove_dir_all(&sdir);
        let _ = std::fs::remove_file(sync_conf_path(&data));
        let _ = std::fs::remove_file(&data);
        Ok(())
    }
}
