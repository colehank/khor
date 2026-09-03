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
        sysinfo::ProcessRefreshKind::nothing().with_cpu().with_memory().with_user(sysinfo::UpdateKind::OnlyIfNotSet),
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
                // Zero means the platform did not say, not "started
                // this instant" — the same absent-is-not-zero rule the
                // GPU columns carry.
                run_secs: match p.run_time() {
                    0 => None,
                    secs => Some(secs),
                },
                cmd: cmdline(p),
            }
        })
        .collect();
    rank(&mut rows, by, desc);
    rows.truncate(n);
    rows
}

/// The command line, joined and cut **here** — on the machine that has
/// it (`khor_core::CMD_CAP`, and see [`khor_core::TopRow::cmd`] for why
/// the cut is not the face's job).
///
/// `None` rather than an empty string when the platform will not spell
/// it: a kernel thread has no command line, and an empty one would draw
/// as a name that failed to load rather than as a process that has
/// none. Cut by `char` so the cut cannot land inside a UTF-8 sequence.
fn cmdline(p: &sysinfo::Process) -> Option<String> {
    cut(&p.cmd().iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" "))
}

/// The cut itself, taking the joined string rather than the process, so
/// it can be tested against a command line longer than anything this
/// machine happens to be running — a cap tested only against whatever
/// `ps` shows today is a cap nothing proves.
fn cut(joined: &str) -> Option<String> {
    if joined.trim().is_empty() {
        return None;
    }
    Some(joined.chars().take(khor_core::CMD_CAP).collect())
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
            cmd: None,
        }
    }

    /// **The command line is cut on the machine that has it**, and the
    /// cut is by character so it cannot land inside a UTF-8 sequence
    /// and produce a row that will not decode.
    ///
    /// Tested against a line longer than anything this machine is
    /// running: a cap checked only against today's `ps` output passes
    /// on a machine with nothing long on it, which is a check that
    /// cannot fail. The multi-byte half is the one that would bite in
    /// production — a path with Chinese in it is ordinary.
    #[test]
    fn a_command_line_is_cut_where_it_lives_and_never_mid_character() {
        let cap = khor_core::CMD_CAP;
        let long = "a".repeat(cap * 3);
        assert_eq!(cut(&long).unwrap().chars().count(), cap, "a long line is cut to the cap");
        let short = "python3 -m vllm.entrypoints.openai.api_server";
        assert_eq!(cut(short).as_deref(), Some(short), "a short one is left alone");
        assert_eq!(cut("").as_deref(), None, "a kernel thread has no command line, and absent is not empty");
        assert_eq!(cut("   ").as_deref(), None, "nor does whitespace count as one");
        // Every char is 3 bytes here, so a byte-wise cut at the cap
        // would slice one in half and the result would not be a string.
        let cjk = "路".repeat(cap * 2);
        let got = cut(&cjk).unwrap();
        assert_eq!(got.chars().count(), cap, "counted in characters, not bytes");
        assert!(got.chars().all(|c| c == '路'), "and every one of them survived whole");
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
