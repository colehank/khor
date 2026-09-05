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
        // **`with_disk_usage` is here and not after the cut**, unlike
        // the command line above it, and the reason is the ranking:
        // 「按读写排的前 200」 has to compare rows that have not been cut
        // yet, so a figure only the survivors carry would rank a column
        // nobody had read.
        //
        // What it costs, measured 2026-09-05 on this Mac (640
        // processes, best of three, whole `sample_top`): **7.05ms
        // without, 7.60ms with** — under a millisecond, so the ask
        // stays where the ranking needs it.
        //
        // **Not measured on Linux**, where it is an extra open-and-read
        // of `/proc/[pid]/io` per process and turing runs 2173 of them.
        // If that ever shows, the shape of the fix is the one above:
        // rank on something else and read this after the cut.
        sysinfo::ProcessRefreshKind::nothing()
            .with_cpu()
            .with_memory()
            .with_disk_usage()
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
    // **Whose khor this is**, read once, so every row's disk figure can
    // be answered before it is read rather than after it comes back a
    // zero that means nothing (`io_readable`). The account is spelled
    // the way sysinfo spells it — a number on unix — because the only
    // thing done with it is comparison.
    let mine: Option<String> = sysinfo::get_current_pid()
        .ok()
        .and_then(|p| sys.process(p))
        .and_then(|p| p.user_id())
        .map(|u| u.to_string());
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
                // **The platform is a parameter, not a `cfg`** — see
                // `status_token`, where the two disagreements that
                // matter are written down.
                status: Some(status_token(p.status(), cfg!(target_os = "macos"), mac_running(pid))),
                // CPU-milliseconds since the process started. Comes
                // with `with_cpu()`, which this refresh already asks
                // for, so it costs nothing beyond the read.
                cpu_ms: Some(p.accumulated_cpu_time()),
                // **Asked only of processes we are allowed to read**:
                // the library answers an unreadable one with a zero,
                // and a zero here is a claim about the process rather
                // than about the permission (`io_readable`).
                io_bytes: io_readable(mine.as_deref(), p.user_id().map(|u| u.to_string()).as_deref())
                    .then(|| {
                        let io = p.disk_usage();
                        io.total_read_bytes.saturating_add(io.total_written_bytes)
                    }),
            }
        })
        .collect();
    rank(&mut rows, by, desc);
    rows.truncate(n);
    fill_args(sys, &mut rows);
    rows
}

/// **One vocabulary for three platforms' words**, and the platform is a
/// parameter so both branches can be read — and tested — from either
/// machine.
///
/// The token that comes out of here is what crosses the wire; the face
/// looks up a Chinese word for it (`zh.toml` `top-status-*`) and prints
/// the token itself when it has none. **Nothing falls through to
/// 「运行」.**
///
/// # The two the variant names get wrong
///
/// Read from sysinfo 0.39's own per-platform sources, not from the
/// variant names, because the names are what mislead:
///
/// - **`Idle`** is `'I'` on Linux — *an idle kernel thread* — and
///   `SIDL` on macOS, which is **a process being created by fork**.
///   Nothing about those two is the same fact, so they do not share a
///   word.
/// - **`Dead`** is `'X'`/`'x'` on Linux, a process that really is
///   finished. On macOS it is what sysinfo hands back for
///   `ThreadStatus::Uninterruptible` — a thread stuck in an
///   uninterruptible wait, which is *Linux's `'D'`*. So macOS's `Dead`
///   and Linux's `UninterruptibleDiskSleep` are the same fact under
///   different names, and macOS's `Dead` is emphatically **not** Linux's.
///
/// # What holds this together over time
///
/// The match is **exhaustive on purpose**. `sysinfo::ProcessStatus` is
/// not `#[non_exhaustive]`, so the day the library grows a variant this
/// file **stops compiling** rather than quietly sorting the new state
/// into whichever arm a wildcard led to. That compiler error is the
/// guarantee that 「映射不到的不许吞成运行」 is kept by something other
/// than somebody remembering it.
///
/// `Unknown(n)` is the one status with no name, and it keeps its number
/// — `unknown-13` on a screen is a thing somebody can look up, where a
/// bare 「未知」 is a dead end.
pub(crate) fn status_token(s: sysinfo::ProcessStatus, mac: bool, running: Option<bool>) -> String {
    use sysinfo::ProcessStatus as S;
    match (s, mac) {
        // **macOS's SRUN does not mean running**, and the library's own
        // repair for it does not fire — see `mac_running`, which is
        // where the second opinion comes from.
        (S::Run, true) => match running {
            Some(true) => "run",
            Some(false) => "sleep",
            None => "unclear",
        },
        (S::Run, false) => "run",
        (S::Sleep, _) => "sleep",
        (S::Stop, _) => "stopped",
        (S::Zombie, _) => "zombie",
        (S::Tracing, _) => "tracing",
        (S::Parked, _) => "parked",
        (S::Wakekill, _) => "wakekill",
        (S::Waking, _) => "waking",
        (S::Suspended, _) => "suspended",
        (S::LockBlocked, _) => "lockblocked",
        (S::UninterruptibleDiskSleep, _) => "diskwait",
        // The two above are the platform's, not the name's.
        (S::Idle, false) => "idle",
        (S::Idle, true) => "spawning",
        (S::Dead, false) => "dead",
        (S::Dead, true) => "diskwait",
        (S::Unknown(n), _) => return format!("unknown-{n}"),
    }
    .to_owned()
}

/// **A second opinion on 「is this process actually running」, asked of
/// macOS only** — because on macOS the first opinion is wrong four
/// times out of five.
///
/// Measured 2026-09-05 on this Mac, against `ps` on the *same* 195
/// pids in the same second:
///
/// | khor said | `ps` said | rows |
/// |---|---|---|
/// | `run` | `S` (sleeping) | **156** |
/// | `sleep` | `S` | 39 |
/// | `run` | `R` | **0** |
///
/// Not one row agreed. The chain: BSD's `p_stat` is `SRUN` for a
/// process that merely *has* a thread able to run, so sysinfo reads
/// `Run` for most of the machine. sysinfo knows this — its comment says
/// 「if the status is `Run`, then it's very likely wrong」 — and repairs
/// it from the thread status. **The repair does not fire**: it asks
/// `proc_pidinfo(PROC_PIDTHREADINFO, arg = 0)`, and that call wants a
/// thread handle, so it fails, and the failure branch **fabricates
/// `ThreadStatus::Running`** for any process that is alive. The wrong
/// answer arrives through the code that exists to prevent it.
///
/// So this asks the task instead: `pti_numrunning` is how many of the
/// process's threads are on a core right now, which is the same thing
/// `ps` prints `R` for.
///
/// `None` means the machine would not say — another account's process,
/// or one that died between the listing and this call — and it stays
/// `None` rather than becoming either word. That is the whole point:
/// **a status nobody could read must not arrive as 「运行」**, which is
/// exactly the trip the library takes.
#[cfg(target_os = "macos")]
fn mac_running(pid: u32) -> Option<bool> {
    let mut ti: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let want = std::mem::size_of::<libc::proc_taskinfo>() as i32;
    // SAFETY: `ti` is a zeroed struct of exactly the size handed to the
    // call, and the kernel writes at most that many bytes into it. A
    // short write is refused below rather than read.
    let got = unsafe {
        libc::proc_pidinfo(pid as i32, libc::PROC_PIDTASKINFO, 0, (&mut ti as *mut libc::proc_taskinfo).cast(), want)
    };
    (got == want).then(|| ti.pti_numrunning > 0)
}

/// Not a question anywhere else: Linux's `/proc/[pid]/stat` state char
/// is the very thing `ps` prints, so `status_token` takes it as it
/// comes.
#[cfg(not(target_os = "macos"))]
fn mac_running(_pid: u32) -> Option<bool> {
    None
}

/// **Whether a disk figure for this process would be a reading or a
/// zero**, decided before the read rather than after it.
///
/// `/proc/<pid>/io` is `0400` and macOS's `proc_pid_rusage` refuses
/// another account's process — and in both cases sysinfo returns
/// **zero**, not an error. A zero that means 「不许看」 sitting in a
/// column that otherwise means 「读了这么多字节」 is the
/// neighbouring-value mistake in its most convincing dress: it is not
/// even a wrong number, it is a *plausible* one.
///
/// So: our own processes, or every process when khor runs as root.
/// Anything else is absent, and absent is what the face draws as an
/// empty cell and what `rank` sinks in both directions.
///
/// Spelled in the account's own string rather than a number because
/// that is the only spelling every platform shares — the comparison is
/// all this does with it. `"0"` is root on both of khor's, and on a
/// platform where accounts are not numbers it is simply an id nobody
/// has, which fails safe.
fn io_readable(mine: Option<&str>, theirs: Option<&str>) -> bool {
    match mine {
        // Root reads everybody's, which is the whole difference.
        Some("0") => true,
        Some(m) => theirs == Some(m),
        // Not knowing which account we are is not permission to guess.
        None => false,
    }
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
            TopBy::CpuTime => r.cpu_ms.map(|v| v as f64),
            // On a shared box this is absent for most rows, because
            // khor may not read another account's — so this column
            // ranks what it may read and sinks the rest, both ways.
            TopBy::Io => r.io_bytes.map(|v| v as f64),
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
            // **Grouping, not ranking** — see the variant. The token is
            // what sorts, not the word the face prints, so a clump
            // never splits along a translation.
            TopBy::Status => a.status.cmp(&b.status),
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
            status: None,
            cpu_ms: None,
            io_bytes: None,
        }
    }

    /// **The two statuses whose meaning changes with the platform**,
    /// pinned in both directions.
    ///
    /// This is the test the mapping exists for. The other eleven arms
    /// are the same word on both machines and a table could carry them;
    /// these two are where reading the variant name instead of the
    /// platform's source produces a **wrong word on screen, with
    /// nothing red anywhere** — macOS rows in the middle of being
    /// forked labelled 「闲置」, and macOS rows stuck in an
    /// uninterruptible wait labelled 「已退出」 while they are very much
    /// running.
    ///
    /// **Both arms of each pair are asserted, not just the interesting
    /// one.** An implementation that answered `spawning` on both
    /// platforms would be exactly as wrong, and half a test would pass
    /// it.
    #[test]
    fn the_two_statuses_that_do_not_mean_the_same_thing_on_both_machines() {
        use sysinfo::ProcessStatus as S;
        // Linux `'I'`: an idle kernel thread. macOS `SIDL`: a process
        // being created by fork. Not the same fact, not the same word.
        assert_eq!(status_token(S::Idle, false, None), "idle", "Linux's I is an idle kernel thread");
        assert_eq!(status_token(S::Idle, true, None), "spawning", "macOS's SIDL is a process being forked");
        // Linux `'X'`: finished. macOS: what sysinfo returns for a
        // thread in uninterruptible wait — which is Linux's `'D'`.
        assert_eq!(status_token(S::Dead, false, None), "dead", "Linux's X really is finished");
        assert_eq!(
            status_token(S::Dead, true, None),
            status_token(S::UninterruptibleDiskSleep, false, None),
            "macOS's Dead is a thread stuck in an uninterruptible wait — Linux calls that D"
        );
        // And the one they do agree on, so the pairs above are read as
        // a difference rather than as this function answering by
        // platform for everything.
        assert_eq!(status_token(S::Run, true, Some(true)), status_token(S::Run, false, None), "running is running");
    }

    /// **The second opinion can tell the two apart**, which is the
    /// only thing that makes it worth asking.
    ///
    /// `status_token`'s macOS arm is a fork on this function's answer,
    /// so an implementation that answered `Some(false)` for everything
    /// would turn every macOS row 「休眠」 — and against a machine where
    /// `ps` says 755 of 762 are sleeping, **that build looks right**.
    /// It would also be the same shape of wrong as the library's
    /// always-`Running`, just pointed the other way.
    ///
    /// So both directions are pinned, and each arm is a fact rather
    /// than a hope:
    ///
    /// - **this test's own process is running** — it is executing this
    ///   line — so `Some(true)` is the only honest answer, and an
    ///   always-`false` or always-`None` build fails here;
    /// - **a pid nobody has is `None`**, so an always-`Some` build
    ///   fails there. Absence has to survive, because it is what keeps
    ///   another account's process from arriving as a word.
    ///
    /// The live control that produced the numbers in `mac_running`'s
    /// head is not in here: it burned a core for fifteen seconds to
    /// make one row say `run`, and a test that needs a spinning CPU to
    /// pass is a test that will fail on a busy machine for a reason
    /// that has nothing to do with the code.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_second_opinion_answers_both_ways() {
        assert_eq!(
            mac_running(std::process::id()),
            Some(true),
            "this very process is on a core — an always-sleeping reading would be indistinguishable from a machine at rest"
        );
        assert_eq!(
            mac_running(4_000_000_000),
            None,
            "and a pid nobody has cannot come back as a word — absence is what protects another account's rows"
        );
    }

    /// **A status with no name keeps its number**, and never borrows a
    /// neighbour's word.
    ///
    /// 「映射不到的不许吞成运行」 has two halves and only one of them is
    /// testable here: that an unknown status comes out as itself. The
    /// other half — that a *new* sysinfo variant cannot silently join
    /// an existing arm — is held by the match being exhaustive over a
    /// non-`non_exhaustive` enum, so it is a **compile** error and not
    /// a test at all. If a wildcard arm ever appears in `status_token`,
    /// that guarantee is gone and this comment is the only thing that
    /// will say so.
    #[test]
    fn a_status_with_no_name_says_so_and_keeps_its_number() {
        use sysinfo::ProcessStatus as S;
        assert_eq!(status_token(S::Unknown(13), false, None), "unknown-13");
        assert_eq!(status_token(S::Unknown(13), true, None), "unknown-13");
        // The point of keeping the number: two unknowns are two
        // different states, and one word for both would hide that.
        assert_ne!(status_token(S::Unknown(13), false, None), status_token(S::Unknown(7), false, None));
    }

    /// **A disk figure khor is not allowed to read is absent, not
    /// zero** — the decision made before the read, because after it the
    /// two are the same bytes.
    ///
    /// The case that matters is the shared box: khor runs as one
    /// account and most of the two hundred rows belong to others.
    /// Without this, every one of those rows would print 「0 B」, which
    /// is a claim about the process (它不碰盘) standing in for a fact
    /// about the permission (我看不到).
    #[test]
    fn a_process_khor_may_not_read_has_no_disk_figure_rather_than_a_zero() {
        assert!(io_readable(Some("501"), Some("501")), "our own process is readable");
        assert!(!io_readable(Some("501"), Some("0")), "root's is not, and a zero would be a lie");
        assert!(!io_readable(Some("501"), Some("502")), "nor another account's");
        assert!(io_readable(Some("0"), Some("502")), "root reads everybody's — that is the difference");
        assert!(!io_readable(None, Some("501")), "not knowing who we are is not permission to guess");
        // A process whose account the machine will not name is not
        // ours by default — the same absent-is-not-a-match rule the
        // rest of these rows follow.
        assert!(!io_readable(Some("501"), None), "and a process with no account named is not ours");
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

    /// The three columns of 2026-09-05 rank by themselves too — and
    /// each is given a shape the *other two* cannot produce, so a
    /// column wired to its neighbour is caught rather than accidentally
    /// right.
    ///
    /// **The pids are laid out so no two of the three answer alike.**
    /// A test where 状态 / CPU时间 / 读写 happened to want the same
    /// order would pass for a `rank` that read any one of them — the
    /// commonest way a new-column test says nothing.
    ///
    /// The 读写 arm carries the extra claim: **a row khor may not read
    /// sinks in both directions.** On a shared box that is most rows,
    /// and a reversal that floated them would answer 「谁读写最少」 with
    /// 「我看不到的那些」.
    #[test]
    fn the_status_the_cpu_time_and_the_disk_total_each_rank_by_themselves() {
        use khor_core::TopBy;
        let mut base =
            vec![row(1, "a", 1.0, 1, None), row(2, "b", 1.0, 1, None), row(3, "c", 1.0, 1, None)];
        // zombie < run < sleep alphabetically, so the words group in an
        // order none of the numbers below would give.
        base[0].status = Some("sleep".into());
        base[1].status = Some("zombie".into());
        base[2].status = Some("run".into());
        base[0].cpu_ms = Some(50);
        base[1].cpu_ms = Some(10);
        base[2].cpu_ms = Some(90);
        base[0].io_bytes = Some(7);
        base[1].io_bytes = None; // another account's — unreadable, not zero
        base[2].io_bytes = Some(3);
        let ranked = |by: TopBy, desc: bool| {
            let mut r = base.clone();
            rank(&mut r, by, desc);
            r.iter().map(|x| x.pid).collect::<Vec<_>>()
        };
        assert_eq!(ranked(TopBy::Status, false), vec![3, 1, 2], "A to Z on the token: run, sleep, zombie");
        assert_eq!(ranked(TopBy::Status, true), vec![2, 1, 3], "and the reader flips which clump is on top");
        assert_eq!(ranked(TopBy::CpuTime, true), vec![3, 1, 2], "most CPU burned first");
        assert_eq!(ranked(TopBy::CpuTime, false), vec![2, 1, 3], "least first");
        assert_eq!(ranked(TopBy::Io, true), vec![1, 3, 2], "most bytes first, the unreadable one last");
        assert_eq!(
            ranked(TopBy::Io, false),
            vec![3, 1, 2],
            "and least first STILL leaves it last — 读不到 is not 最少的"
        );
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

