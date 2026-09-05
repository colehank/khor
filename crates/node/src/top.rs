//! What is eating this machine right now: the processes at the top of
//! the CPU, the answer 进程 top on the machine card draws
//! (2026-09-02:「环下面显示进程top」).
//!
//! **Read-only, and staying that way.** No kill, no renice, no signal:
//! khor's card answers 这台机器在忙什么, and the verbs that change a
//! machine belong to a terminal a person opened on purpose (`开终端`
//! is one press away on the same card). A kill button beside a reading
//! is the reading daring you to use it.
//!
//! # Which way this can be wrong
//!
//! - **The window is "since the last ask", not a fixed second.** The
//!   same [`sysinfo`]-held-delta design as `vitals::Sampler`, and the
//!   same reason: a rate needs two looks, and whoever asks decides the
//!   gap. The first ask in a process pays
//!   [`sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`] once so a one-shot
//!   answer is a rate rather than a lifetime average — the `ps` trap
//!   (`%cpu` there is a decaying average on macOS and a lifetime one
//!   on Linux) that made shelling out the wrong road.
//! - **A figure is per core**, the way `top` prints it: a process
//!   holding four cores reads 400, not 40 — capping it at 100 would
//!   make the busiest kind of process read like an ordinary one.
//! - **Short-lived processes are invisible**: something that starts
//!   and dies inside the window contributes nothing. That is every
//!   sampler's blindness, and a card polling every few beats inherits
//!   it honestly rather than pretending to be an audit.

use std::sync::{Mutex, OnceLock};

pub use khor_core::TopRow;

mod gpu;

/// The process table's own `System`, apart from the vitals sampler's on
/// purpose: the two answer on different beats, and sharing one would
/// let either ask reset the other's CPU window (the shared-window trap
/// `vitals::gpu::mac` documents from the machine's other end).
static SAMPLER: OnceLock<Mutex<(sysinfo::System, sysinfo::Users)>> = OnceLock::new();

fn refresh(sys: &mut sysinfo::System) {
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        // Dead processes leave the table — a row for a pid that is gone
        // would be a reading of nothing.
        true,
        // **No `with_cmd` here, and that is the point.** The refresh
        // kind is `nothing()` plus exactly what is asked for, so a
        // field nobody asks for comes back empty rather than missing —
        // which is why the command line read as absent everywhere until
        // it was asked for (measured 2026-09-04 over the wire). But
        // asking for it *here* asks it of every process on the machine,
        // every five seconds, to show nine rows: 2173 of them on turing
        // today. The nine that survive the cut are the only ones whose
        // command line anybody reads, so the ask moved after the cut
        // (`fill_args`).
        sysinfo::ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_user(sysinfo::UpdateKind::OnlyIfNotSet),
    );
}

/// The account behind a process, by name when the machine can spell it.
/// The user list is read once and re-read only on a miss — an account
/// created since start is the one case a stale list answers wrongly,
/// and a miss is exactly when it shows.
fn user_of(users: &mut sysinfo::Users, p: &sysinfo::Process) -> String {
    let Some(uid) = p.user_id() else { return String::new() };
    if let Some(u) = users.get_user_by_id(uid) {
        return u.name().to_owned();
    }
    users.refresh();
    match users.get_user_by_id(uid) {
        Some(u) => u.name().to_owned(),
        None => uid.to_string(),
    }
}

/// The `n` processes at the top of `by`, ranked **here**, on the
/// machine the rows are about.
///
/// **Ranking travels; rows do not.** A shared box runs ten thousand
/// processes (measured on turing), and a face that wanted them ordered
/// by memory used to have two bad choices: pull all ten thousand every
/// few seconds to show nine, or rank a cut that was cut by something
/// else — the process holding the most memory is rarely among the
/// hungriest by CPU. Ranking where the rows are makes the answer both
/// true and small (`khor_core::TopBy`).
///
/// **A column this machine cannot read sinks, both ways.** Sorting by
/// GPU on a Mac must not promote the rows whose GPU is unknown to the
/// top of "least busy" — absence is not a small number, so it is last
/// in either direction and the pid breaks the tie so two asks in one
/// quiet second agree.
///
/// **Blocking** — the one caller inside async code goes through
/// `spawn_blocking`, the vitals rule.
pub fn sample_top(n: usize, by: khor_core::TopBy, desc: bool) -> Vec<TopRow> {
    let mut held = SAMPLER
        .get_or_init(|| {
            let mut sys = sysinfo::System::new();
            // The first look, so the reading below has something to
            // subtract — `vitals::Sampler`'s arrangement.
            refresh(&mut sys);
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            Mutex::new((sys, sysinfo::Users::new_with_refreshed_list()))
        })
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let (sys, users) = &mut *held;
    refresh(sys);
    // What the cards say about each pid, asked once for the whole
    // table: NVML is a driver conversation, and one per row would be
    // hundreds of them for one screen.
    let cards = gpu::by_pid();
    let mut rows: Vec<TopRow> = sys
        .processes()
        .values()
        .map(|p| {
            let pid = p.pid().as_u32();
            let on_card = cards.get(&pid).copied().unwrap_or_default();
            TopRow {
                pid,
                name: p.name().to_string_lossy().into_owned(),
                user: user_of(users, p),
                cpu_pct: p.cpu_usage(),
                mem: p.memory(),
                gpu_pct: on_card.pct,
                gpu_mem: on_card.mem,
                run_secs: age_of(p),
                // Filled after the cut — see `fill_args`.
                arg: None,
                // **Whose child this is**, so the face can tell a pool
                // of ten identical workers from ten unrelated
                // processes that happen to share a name. `sysinfo`
                // answers this on all three platforms and costs
                // nothing extra: it is already in the process record
                // this loop is walking.
                ppid: p.parent().map(|x| x.as_u32()),
            }
        })
        .collect();
    rank(&mut rows, by, desc);
    rows.truncate(n);
    fill_args(sys, &mut rows);
    rows
}

/// How long a process has been up, or `None` when the machine cannot
/// honestly say.
///
/// **`run_time()` is not safe to send as it stands**, measured
/// 2026-09-04 over the wire: on this Mac, `launchd` and `logd` came back
/// with **1788456301 seconds — 56 years**, which is the current unix
/// time, not an age. sysinfo derives the age by subtracting the start
/// time, and for a process it cannot read the start time of (the
/// root-owned ones, on macOS) that start time is `0`, so the
/// subtraction hands back the clock. The column would have printed
/// 「20699 天」 next to a machine that has been up five days.
///
/// So the two ways it can be wrong are both refused rather than shown:
///
/// - a start time of `0` is 「没读到」, and anything derived from it is
///   arithmetic on a missing number;
/// - an age longer than the machine's own uptime cannot be true of a
///   process running on it, whatever the platform says. This is the
///   general form — it catches the same lie on a platform that spells
///   its missing start time as something other than zero.
///
/// Absent, never zero: `0 秒` would name the oldest process on the box
/// as its newest, which is the neighbouring-value trap in its most
/// convincing dress.
fn age_of(p: &sysinfo::Process) -> Option<u64> {
    age(p.start_time(), p.run_time(), sysinfo::System::uptime())
}

/// The decision itself, on three numbers, so the measured failure can
/// be a test rather than a memory — see [`age_of`].
fn age(start_time: u64, run_time: u64, uptime: u64) -> Option<u64> {
    if start_time == 0 {
        return None;
    }
    (run_time > 0 && run_time <= uptime).then_some(run_time)
}

/// Reads the command line **of the rows that survived the cut**, and
/// puts each one's identifying word on it.
///
/// Asking for the command line is a per-process read (`/proc/[pid]/
/// cmdline` and its equivalents), so what it costs is set by how many
/// processes are asked. Measured on this Mac, 640 processes: asking all
/// of them costs about the same as the reading refresh itself, while
/// asking only two hundred took **2.15ms**. turing runs 2173 — the
/// whole point of ranking on the machine is that only the cut crosses
/// the wire, and only the cut needs this either.
fn fill_args(sys: &mut sysinfo::System, rows: &mut [khor_core::TopRow]) {
    if rows.is_empty() {
        return;
    }
    let pids: Vec<sysinfo::Pid> = rows.iter().map(|r| sysinfo::Pid::from_u32(r.pid)).collect();
    // `false`: this is a second look at processes already in the table,
    // not a census — a process that died since the ranking should not
    // take the whole table's rows with it.
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&pids),
        false,
        sysinfo::ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::Always),
    );
    for r in rows.iter_mut() {
        let Some(p) = sys.process(sysinfo::Pid::from_u32(r.pid)) else { continue };
        let argv: Vec<String> = p.cmd().iter().map(|a| a.to_string_lossy().into_owned()).collect();
        r.arg = pick_arg(&argv);
    }
}

/// The word that says which program this is: **the first argument after
/// the executable that is not a flag**.
///
/// One rule, and it lands on the identifying part of the three shapes a
/// process is usually started in:
///
/// - `python -m vllm.entrypoints.openai.api_server` → the module
/// - `python train.py --config x` → the script
/// - `cargo build --release` → the subcommand
///
/// **`argv[0]` is skipped** because it is the interpreter, and the
/// interpreter is exactly the word `name` already said — keeping it
/// would hand back `python3` for every one of the rows this field
/// exists to tell apart.
///
/// **A bare number is never that word**, so it is skipped too. A flag
/// that takes a value cannot be told from a positional argument, and
/// the values that leak through are overwhelmingly numeric — measured
/// 2026-09-04 across 200 rows on each machine: `go.d.plugin 1`,
/// `apps.plugin 1`, `nvidia-smi 5`, `systemd 76`, **44 of hinton's 158
/// answers**. A number sitting where a program name goes reads as a
/// fact about the program, and it is not one; nothing at all is the
/// truer answer, and leaves the row saying `go.d.plugin`.
///
/// **What it still does not do**: `rustc --crate-name khor_core
/// crates/…/lib.rs` answers `khor_core` rather than the path, because
/// that flag's value is a word. Knowing better would mean knowing every
/// program's flags; the alternative rule — skip a flag *and* the word
/// after it — breaks the `-m` case above, where the word after the flag
/// is the whole answer. This is the trade, taken deliberately.
fn pick_arg(argv: &[String]) -> Option<String> {
    let word = argv
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-') && !a.trim().is_empty() && !a.chars().all(|c| c.is_ascii_digit()))?;
    Some(word.chars().take(khor_core::ARG_CAP).collect())
}

/// Puts the rows in the asked order. Separate from the sampling so it
/// can be tested without a machine under it.
pub(crate) fn rank(rows: &mut [TopRow], by: khor_core::TopBy, desc: bool) {
    use khor_core::TopBy;
    // Absent readings last whichever way the column is read: `None` is
    // "this machine cannot say", and letting it sort as a zero would
    // turn 没看到 into 最小的 the moment somebody reverses the column.
    let num = |r: &TopRow| -> Option<f64> {
        match by {
            TopBy::Cpu => Some(r.cpu_pct as f64),
            TopBy::Mem => Some(r.mem as f64),
            TopBy::Gpu => r.gpu_pct.map(|v| v as f64),
            TopBy::Vram => r.gpu_mem.map(|v| v as f64),
            // A process whose age the machine will not say sinks both
            // ways, like the card columns: unknown is not 「刚起的」.
            TopBy::Time => r.run_secs.map(|v| v as f64),
            _ => None,
        }
    };
    rows.sort_by(|a, b| {
        let order = match by {
            // CPU first, memory breaking the tie: an idle machine still
            // ranks by something a person recognises.
            TopBy::Busiest => b.cpu_pct.total_cmp(&a.cpu_pct).then(b.mem.cmp(&a.mem)),
            // The pid is a number, and the pid is also the tiebreak
            // every other column falls back to — so ranking *by* it
            // needs no second key, and reversing it is the only place
            // the tiebreak itself turns over.
            TopBy::Pid => a.pid.cmp(&b.pid),
            TopBy::Name => a.name.cmp(&b.name),
            TopBy::User => a.user.cmp(&b.user),
            _ => match (num(a), num(b)) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (None, None) => std::cmp::Ordering::Equal,
                (None, _) => return std::cmp::Ordering::Greater,
                (_, None) => return std::cmp::Ordering::Less,
            },
        };
        // `Busiest` is already biggest-first; the others are read the
        // way the caller asked.
        let order = if desc && !matches!(by, TopBy::Busiest) { order.reverse() } else { order };
        order.then(a.pid.cmp(&b.pid))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str, cpu: f32, mem: u64, gpu: Option<f32>) -> TopRow {
        TopRow {
            pid,
            name: name.to_owned(),
            user: "zgh".to_owned(),
            cpu_pct: cpu,
            mem,
            gpu_pct: gpu,
            gpu_mem: gpu.map(|_| 1),
            run_secs: None,
            arg: None,
            ppid: None,
        }
    }

    /// **The clock is not an age**, and this is the reading that proved
    /// it rather than a worry about one.
    ///
    /// Measured 2026-09-04 over the wire, before the guard existed: this
    /// Mac reported `launchd` and `logd` at **1788456507 seconds**,
    /// which is not 56 years of uptime — it is the current unix time.
    /// sysinfo subtracts a start time it could not read (`0` for the
    /// root-owned processes on macOS) and hands back the clock, and the
    /// column would have printed 「20699 天」 on a machine five days up.
    ///
    /// The uptime ceiling is the general half: whatever a platform
    /// spells its missing start time as, **a process cannot have been
    /// running longer than the machine it is running on.**
    #[test]
    fn a_clock_is_not_an_age() {
        // The machine as it stood when this was measured: up five days
        // and change, with `loginwindow` running for all of it.
        let up = 447_400;
        assert_eq!(age(0, 1_788_456_507, up), None, "the real reading that started this: unreadable start time");
        assert_eq!(age(1, 1_788_456_507, up), None, "and the same lie with a start time that is merely wrong");
        assert_eq!(age(1_788_000_000, 447_360, up), Some(447_360), "a process up as long as the box is fine");
        assert_eq!(age(1_788_000_000, up, up), Some(up), "including one that has been up for exactly all of it");
        assert_eq!(age(1_788_000_000, up + 1, up), None, "one second older than the machine cannot be true");
        assert_eq!(age(1_788_000_000, 3, up), Some(3), "so is one that started three seconds ago");
        assert_eq!(age(1_788_000_000, 0, up), None, "and zero is absent, not 刚起的");
    }

    /// **The identifying word actually arrives on a real reading.**
    ///
    /// `pick_arg` can be perfect while the column stays empty, and that
    /// is exactly what happened on 2026-09-04: the command line is only
    /// populated for the processes a refresh was asked about, so the
    /// rule had nothing to pick from. A unit test on the rule cannot
    /// see that — only a reading of this machine can.
    ///
    /// Asserted the way `vitals` asserts: the half that holds on every
    /// machine. **This process is in its own table and it has an
    /// argument** (the test binary is run with a filter), so at least
    /// one row must carry a word.
    #[test]
    fn the_identifying_word_reaches_a_real_reading() {
        let rows = sample_top(200, khor_core::TopBy::Busiest, true);
        assert!(!rows.is_empty(), "probe dead: no rows at all");
        let with = rows.iter().filter(|r| r.arg.is_some()).count();
        assert!(
            with > 0,
            "not one of {} rows carries an identifying word — the command line was never asked for",
            rows.len()
        );
    }

    /// **The word that tells two `python3` rows apart**, on the shapes
    /// a process is actually started in.
    ///
    /// The first case is the one this field exists for: three vLLM
    /// workers on turing, all named `python3`, holding 21.7G / 10.9G /
    /// 10.3G of card memory.
    #[test]
    fn the_identifying_word_is_the_first_argument_that_is_not_a_flag() {
        let argv = |s: &str| s.split(' ').map(str::to_owned).collect::<Vec<_>>();
        let pick = |s: &str| pick_arg(&argv(s));
        assert_eq!(pick("python -m vllm.entrypoints.openai.api_server").as_deref(), Some("vllm.entrypoints.openai.api_server"));
        assert_eq!(pick("python train.py --config big.yaml").as_deref(), Some("train.py"));
        assert_eq!(pick("cargo build --release").as_deref(), Some("build"));
        // argv[0] is the interpreter, and the interpreter is the word
        // `name` already said — handing it back would answer `python3`
        // for exactly the rows this is meant to separate.
        assert_eq!(pick("python3").as_deref(), None, "an executable with no arguments identifies nothing further");
        assert_eq!(pick("khor serve").as_deref(), Some("serve"));
        assert_eq!(pick("nginx -g daemon off;").as_deref(), Some("daemon"), "a flag's value counts when nothing else does");
        // **A bare number is never the word.** These are the real
        // readings the rule leaked before it skipped them — 44 of
        // hinton's 158 answers were one of these.
        assert_eq!(pick("sleep 5").as_deref(), None, "「5」 says nothing about which program this is");
        assert_eq!(pick("/lib/systemd/systemd --system --deserialize 76").as_deref(), None, "systemd, not systemd 76");
        assert_eq!(pick("go.d.plugin 1").as_deref(), None);
        assert_eq!(pick("python 2to3.py").as_deref(), Some("2to3.py"), "a word that merely starts with a digit stays");
        assert_eq!(pick_arg(&[]), None, "a kernel thread has no command line at all");
        assert_eq!(pick_arg(&["python".into(), "--verbose".into()]).as_deref(), None, "flags only: nothing to say");
        // **The rule's own limit, pinned so it is a decision rather
        // than a surprise**: a flag that takes a value is not known to
        // be one, so this lands on the value rather than the path
        // after it. Fixing that would break the `-m` case above, where
        // the value *is* the answer.
        assert_eq!(
            pick("rustc --crate-name khor_core crates/core/src/lib.rs").as_deref(),
            Some("khor_core"),
            "the documented trade: a flag's value cannot be told from a positional argument"
        );
        // Counted in characters, not bytes: a byte-wise cut would slice
        // a multi-byte character in half and the row would not decode.
        let cap = khor_core::ARG_CAP;
        let long = format!("python /{}", "路".repeat(cap * 2));
        let got = pick(&long).unwrap();
        assert_eq!(got.chars().count(), cap, "a very long token is cut to the cap");
        assert!(got.chars().skip(1).all(|c| c == '路'), "and every character survived whole");
    }

    /// The age column obeys the same rule as the card columns: **a
    /// process whose age the machine will not say sinks both ways.**
    ///
    /// Reversing is the only thing that tells the two apart. Sorted
    /// youngest-first, a `None` that ranked as a zero would sit at the
    /// very top and read as 刚起的 — the newest process on the machine
    /// — which is the neighbouring-value mistake in its most convincing
    /// form, because that row looks exactly like a true answer.
    #[test]
    fn an_age_the_machine_will_not_say_sinks_either_way() {
        use khor_core::TopBy;
        let aged = |pid: u32, secs: Option<u64>| {
            let mut r = row(pid, "p", 1.0, 10, None);
            r.run_secs = secs;
            r
        };
        let mut rows = vec![aged(1, Some(60)), aged(2, None), aged(3, Some(86_400))];
        rank(&mut rows, TopBy::Time, true);
        assert_eq!(
            rows.iter().map(|r| r.pid).collect::<Vec<_>>(),
            vec![3, 1, 2],
            "longest-running first, and the one with no age is last"
        );
        rank(&mut rows, TopBy::Time, false);
        assert_eq!(
            rows.iter().map(|r| r.pid).collect::<Vec<_>>(),
            vec![1, 3, 2],
            "youngest first — and the ageless one is STILL last, not passing itself off as the newest"
        );
    }

    /// **A column with holes puts the holes last, whichever way it is
    /// read.**
    ///
    /// This is the one property of the ranking that cannot be seen by
    /// looking at a screen: sorted one way the absent rows are at the
    /// bottom because they are "smallest", and a reader would believe
    /// it. Reversing is what tells the two apart — and a `None` that
    /// sorted as a zero would climb to the top of "least busy" and
    /// report 没看到 as 最闲的.
    #[test]
    fn a_reading_this_machine_cannot_take_sinks_either_way() {
        use khor_core::TopBy;
        let mut rows = vec![
            row(1, "a", 1.0, 10, Some(50.0)),
            row(2, "b", 2.0, 20, None),
            row(3, "c", 3.0, 30, Some(90.0)),
        ];
        rank(&mut rows, TopBy::Gpu, true);
        assert_eq!(
            rows.iter().map(|r| r.pid).collect::<Vec<_>>(),
            vec![3, 1, 2],
            "biggest first, and the one with no reading is last"
        );
        rank(&mut rows, TopBy::Gpu, false);
        assert_eq!(
            rows.iter().map(|r| r.pid).collect::<Vec<_>>(),
            vec![1, 3, 2],
            "smallest first — and the one with no reading is STILL last, not first"
        );
    }

    /// Each column ranks by itself, and the node's own order is not
    /// any of them: `Busiest` is CPU with memory breaking the tie, and
    /// that is what a frame with no column asked for decodes to.
    #[test]
    fn every_column_ranks_by_its_own_number() {
        use khor_core::TopBy;
        // The busiest by CPU is not the biggest by memory — which is the
        // whole reason ranking travels to the machine.
        let base =
            vec![row(1, "big-mem", 1.0, 900, None), row(2, "busy", 90.0, 10, None), row(3, "mid", 50.0, 500, None)];
        let ranked = |by: TopBy, desc: bool| {
            let mut r = base.clone();
            rank(&mut r, by, desc);
            r.iter().map(|x| x.pid).collect::<Vec<_>>()
        };
        assert_eq!(ranked(TopBy::Cpu, true), vec![2, 3, 1]);
        assert_eq!(ranked(TopBy::Mem, true), vec![1, 3, 2], "memory ranks a different machine's worth of rows");
        assert_eq!(ranked(TopBy::Busiest, true), vec![2, 3, 1], "the node's own order is CPU first");
        assert_eq!(ranked(TopBy::Name, false), vec![1, 2, 3], "A to Z: big-mem, busy, mid");
    }

    /// The reading is of a real machine, so the assertions are the ones
    /// that hold on every machine (`vitals`' rule) — and the positive
    /// half is this very test's own spin: a thread this process pins
    /// must put this process on its own top, with a rate that says so.
    /// That is the finder proving itself before anything trusts a
    /// quiet list.
    #[test]
    fn the_top_finds_the_process_doing_the_work() {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let spin = std::thread::spawn(move || {
            let mut n: u64 = 0;
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                n = n.wrapping_add(1);
            }
            n
        });
        // A window for the spin to be visible in: the first ask arms
        // the sampler, the second reads a real delta.
        let _ = sample_top(8, khor_core::TopBy::Busiest, true);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let rows = sample_top(usize::MAX, khor_core::TopBy::Busiest, true);
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = spin.join();

        assert!(!rows.is_empty(), "a live machine runs processes");
        let me = std::process::id();
        let mine = rows.iter().find(|r| r.pid == me).expect("this very process is on the table");
        assert!(
            mine.cpu_pct > 0.0,
            "one thread was pinned for the whole window and reads {}",
            mine.cpu_pct
        );
        assert!(mine.mem > 0, "a running process holds memory");
        assert!(!mine.name.is_empty(), "a process has a name");
        // The account is this test's own, which the platform can always
        // spell — so an empty word here is the column not being asked
        // for, not a machine that would not say.
        assert!(!mine.user.is_empty(), "this process's own account is spellable");
        assert!(
            !mine.user.chars().all(|c| c.is_ascii_digit()),
            "the account has a name, not just an id: {}",
            mine.user
        );
        for r in &rows {
            assert!(r.cpu_pct >= 0.0, "a rate is not negative: {} on {}", r.cpu_pct, r.name);
        }
    }

    /// The cut ranks by hunger: asking for one row returns the top of
    /// the same table, not an arbitrary member of it.
    #[test]
    fn the_cut_keeps_the_hungriest() {
        let all = sample_top(usize::MAX, khor_core::TopBy::Busiest, true);
        let one = sample_top(1, khor_core::TopBy::Busiest, true);
        assert_eq!(one.len(), 1.min(all.len()));
        if let (Some(first), Some(head)) = (one.first(), all.first()) {
            // Two asks are two windows, so the *figures* may differ;
            // what must hold is that a single-row answer is a top, not
            // a tail: its one row carries at least as much CPU as the
            // full table's median.
            let mut cpus: Vec<f32> = all.iter().map(|r| r.cpu_pct).collect();
            cpus.sort_by(f32::total_cmp);
            let median = cpus[cpus.len() / 2];
            assert!(
                first.cpu_pct >= median || first.pid == head.pid,
                "a one-row top ({} at {}) reads under the median {}",
                first.name,
                first.cpu_pct,
                median
            );
        }
    }
}
