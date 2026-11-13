//! Admission and evidence for p1s's original one-CPU, two-contender load.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::scratch::Reaped;

const REVISION: &str = "checked_affinity_v3";

fn allowed(text: &str) -> Result<Vec<usize>, String> {
    let list = text
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .ok_or("missing Cpus_allowed_list")?
        .trim();
    let mut cpus = BTreeSet::new();
    for item in list.split(',') {
        let (lo, hi) = item.split_once('-').unwrap_or((item, item));
        let lo: usize = lo.parse().map_err(|_| "invalid CPU ID")?;
        let hi: usize = hi.parse().map_err(|_| "invalid CPU range")?;
        if hi < lo || hi.saturating_sub(lo) > 65535 {
            return Err("invalid CPU range".to_owned());
        }
        cpus.extend(lo..=hi);
    }
    if cpus.is_empty() {
        return Err("empty CPU set".to_owned());
    }
    Ok(cpus.into_iter().collect())
}

fn current_allowed() -> Result<Vec<usize>, String> {
    allowed(&proc_text("/proc/thread-self/status")?)
}

fn proc_text(path: impl AsRef<std::path::Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))
}

#[derive(Debug)]
struct Process {
    pid: u32,
    start: u64,
    ticks: u64,
    threads: Vec<(u32, Vec<usize>)>,
}

fn process_times(pid: u32) -> Result<(u64, u64), String> {
    let stat = proc_text(format!("/proc/{pid}/stat"))?;
    // The comm field may itself contain spaces and parentheses.
    let fields: Vec<_> = stat
        .rsplit_once(") ")
        .ok_or("missing stat comm terminator")?
        .1
        .split_whitespace()
        .collect();
    if matches!(fields.first().copied(), Some("Z" | "X")) {
        return Err(format!("process {pid} is no longer running"));
    }
    let field = |index: usize| -> Result<u64, String> {
        fields
            .get(index)
            .ok_or("short proc stat")?
            .parse()
            .map_err(|_| "invalid proc stat number".to_owned())
    };
    // After comm: state is field3/index0; utime14, stime15, starttime22.
    let ticks = field(11)?
        .checked_add(field(12)?)
        .ok_or("CPU ticks overflow")?;
    Ok((field(19)?, ticks))
}

fn task_ids(pid: u32) -> Result<BTreeSet<u32>, String> {
    fs::read_dir(format!("/proc/{pid}/task"))
        .map_err(|e| format!("list /proc/{pid}/task: {e}"))?
        .map(|entry| {
            entry
                .map_err(|e| format!("entry in /proc/{pid}/task: {e}"))?
                .file_name()
                .to_string_lossy()
                .parse()
                .map_err(|_| "invalid thread ID".to_owned())
        })
        .collect()
}

fn thread_masks(pid: u32) -> Result<Vec<(u32, Vec<usize>)>, String> {
    let began = Instant::now();
    loop {
        let mut observed = BTreeMap::new();
        for tid in task_ids(pid)? {
            let path = format!("/proc/{pid}/task/{tid}/status");
            match fs::read_to_string(&path) {
                Ok(status) => {
                    observed.insert(tid, allowed(&status)?);
                }
                // A resident may finish a blocking worker between the
                // directory snapshot and its status read. Only an absent
                // task may be skipped, never an unreadable live thread.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("read {path}: {e}")),
            }
        }
        let live = task_ids(pid)?;
        if live.is_empty() {
            return Err("process has no observable thread".to_owned());
        }
        if live.iter().all(|tid| observed.contains_key(tid)) {
            return Ok(live
                .into_iter()
                .map(|tid| (tid, observed.remove(&tid).unwrap()))
                .collect());
        }
        // Every task still listed must have a real mask. New arrivals or
        // a vanished status still present in the listing require a fresh
        // complete scan; sustained churn fails closed within 100 ms.
        if began.elapsed() >= Duration::from_millis(100) {
            return Err(format!(
                "no complete live-thread snapshot for resident {pid}"
            ));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

impl Process {
    fn read(pid: u32) -> Result<Self, String> {
        let (start, _) = process_times(pid)?;
        let threads = thread_masks(pid)?;
        let (after, ticks) = process_times(pid)?;
        if after != start {
            return Err(format!(
                "resident {pid} changed identity during observation"
            ));
        }
        Ok(Self {
            pid,
            start,
            ticks,
            threads,
        })
    }

    fn on(&self, cpu: usize) -> bool {
        self.threads
            .iter()
            .all(|(_, cpus)| cpus.as_slice() == [cpu])
    }

    fn record(&self) -> Value {
        json!({"pid": self.pid, "start_ticks": self.start, "cpu_ticks": self.ticks, "threads": self.threads, "thread_snapshot_revision": "exit_aware_v1"})
    }
}

fn pin(command: &str, residents: &[u32], cpu: usize) -> Result<(), String> {
    for pid in residents {
        if *pid == 0 || *pid == std::process::id() {
            return Err("only explicit child resident PIDs may be pinned".to_owned());
        }
        let output = Command::new(command)
            .args(["-a", "-p", "-c", &cpu.to_string(), &pid.to_string()])
            .output()
            .map_err(|e| format!("resident affinity command: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "resident affinity command failed: {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if !Process::read(*pid)?.on(cpu) {
            return Err(format!(
                "resident {pid} did not bind all its threads to CPU {cpu}"
            ));
        }
    }
    Ok(())
}

pub(super) struct BusyCpu {
    cpu: usize,
    allowed: Vec<usize>,
    residents: Vec<Process>,
    contenders: Vec<Reaped>,
    previous: Vec<Process>,
    began: Instant,
}

impl BusyCpu {
    pub(super) async fn start(residents: &[u32]) -> Result<Self, String> {
        let allowed = current_allowed()?;
        // Paired sequential runs use the same observed allowed CPU ID,
        // rather than a different core chosen by each harness PID.
        let cpu = allowed[allowed.len() / 2];
        Self::start_on(residents, cpu).await
    }

    async fn start_on(residents: &[u32], cpu: usize) -> Result<Self, String> {
        let allowed = current_allowed()?;
        if !allowed.contains(&cpu) {
            return Err("selected CPU is not allowed".to_owned());
        }
        if residents.is_empty() {
            return Err("CPU load needs its resident subjects".to_owned());
        }
        pin("taskset", residents, cpu)?;
        let residents: Vec<_> = residents
            .iter()
            .map(|pid| Process::read(*pid))
            .collect::<Result<_, _>>()?;
        let mut owner = Self {
            cpu,
            allowed,
            residents,
            contenders: Vec::new(),
            previous: Vec::new(),
            began: Instant::now(),
        };
        for _ in 0..2 {
            let child = Command::new("taskset")
                .args(["-c", &cpu.to_string(), "sh", "-c", "while :; do :; done"])
                .spawn()
                .map_err(|e| format!("CPU contender spawn failed: {e}"))?;
            owner.contenders.push(Reaped(child));
        }
        let began = Instant::now();
        loop {
            let samples = owner.samples()?;
            let ready = samples.iter().all(|p| p.on(cpu)) && owner.exec_ready()?;
            if ready {
                if !owner.previous.is_empty()
                    && owner.previous.iter().zip(&samples).all(|(before, now)| {
                        now.pid == before.pid
                            && now.start == before.start
                            && now.ticks > before.ticks
                    })
                {
                    owner.previous = samples;
                    owner.log("admitted");
                    return Ok(owner);
                }
                if owner.previous.is_empty() {
                    owner.previous = samples;
                }
            }
            if began.elapsed() >= Duration::from_secs(2) {
                return Err("CPU contenders did not execute on the selected CPU".to_owned());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn exec_ready(&self) -> Result<bool, String> {
        for contender in &self.contenders {
            let bytes = fs::read(format!("/proc/{}/cmdline", contender.0.id()))
                .map_err(|e| format!("read contender {}/cmdline: {e}", contender.0.id()))?;
            let args: Vec<_> = bytes
                .split(|b| *b == 0)
                .filter(|arg| !arg.is_empty())
                .collect();
            if args.as_slice()
                != [
                    b"sh".as_slice(),
                    b"-c".as_slice(),
                    b"while :; do :; done".as_slice(),
                ]
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn samples(&mut self) -> Result<Vec<Process>, String> {
        for resident in &mut self.residents {
            let now = Process::read(resident.pid)?;
            if now.start != resident.start || !now.on(self.cpu) {
                return Err(format!(
                    "resident {} changed identity or CPU affinity",
                    resident.pid
                ));
            }
            *resident = now;
        }
        let mut samples = Vec::new();
        for contender in &mut self.contenders {
            if contender.0.try_wait().map_err(|e| e.to_string())?.is_some() {
                return Err("CPU contender exited before measurement ended".to_owned());
            }
            samples.push(Process::read(contender.0.id())?);
        }
        Ok(samples)
    }

    fn log(&self, phase: &str) {
        // SAFETY: sysconf reads a process-independent constant and takes no pointer.
        let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        assert!(ticks_per_second > 0, "observable kernel CPU tick rate");
        eprintln!(
            "[netlab-cpu] {}",
            json!({
                "schema": 1, "measurement_revision": REVISION, "phase": phase,
                "cpu": self.cpu, "allowed_ids": self.allowed, "cpu_selection": "allowed_midpoint_v1",
            "elapsed_ms": self.began.elapsed().as_millis(), "ticks_per_second": ticks_per_second,
                "residents": self.residents.iter().map(Process::record).collect::<Vec<_>>(),
                "contenders": self.previous.iter().map(Process::record).collect::<Vec<_>>(),
            })
        );
    }

    pub(super) fn verify(&mut self, phase: &str) -> Result<(), String> {
        let samples = self.samples()?;
        if !self.exec_ready()? || samples.len() != 2 || !samples.iter().all(|p| p.on(self.cpu)) {
            return Err("CPU contender execution or affinity changed".to_owned());
        }
        for (before, now) in self.previous.iter().zip(&samples) {
            if now.pid != before.pid || now.start != before.start || now.ticks <= before.ticks {
                return Err("a CPU contender did not make real CPU progress".to_owned());
            }
        }
        self.previous = samples;
        self.log(phase);
        Ok(())
    }
}

#[test]
#[ignore = "separate CPU calibration; never alongside timed netlab"]
fn allowed_ids_are_explicit_and_invalid_lists_fail_closed() {
    assert_eq!(
        allowed("Cpus_allowed_list:\t3,7-8,12\n").unwrap(),
        [3, 7, 8, 12]
    );
    for bad in [
        "",
        "Cpus_allowed_list:",
        "Cpus_allowed_list: 5-3",
        "Cpus_allowed_list: x",
        "Cpus_allowed_list: 1,,2",
    ] {
        assert!(allowed(bad).is_err(), "must reject {bad:?}");
    }
}

#[tokio::test]
#[ignore = "separate CPU calibration; never alongside timed netlab"]
async fn checked_load_proves_real_cpu_progress_and_reaps_its_contenders() {
    let first = Reaped(Command::new("sleep").arg("10").spawn().unwrap());
    let second = Reaped(Command::new("sleep").arg("10").spawn().unwrap());
    let mut load = BusyCpu::start(&[first.0.id(), second.0.id()])
        .await
        .unwrap();
    let pids: Vec<_> = load.contenders.iter().map(|p| p.0.id()).collect();
    tokio::time::sleep(Duration::from_millis(300)).await;
    load.verify("calibration").unwrap();
    drop(load);
    assert!(
        pids.iter()
            .all(|pid| !std::path::Path::new(&format!("/proc/{pid}")).exists())
    );
    crate::lab::result(
        "cpu-admit",
        true,
        "two checked residents and two real CPU contenders on one allowed CPU; contenders reaped",
    );
}

/// The sleep-child calibration does not exercise a resident's thread
/// lifecycle. Admit the exact load against real isolated khor residents
/// before spending the original 600-second measurement window.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "separate real-resident CPU calibration; never alongside timed netlab"]
async fn checked_load_admits_real_residents_and_they_answer() {
    let pair = super::pair("cpu-node", async |_, _| {}).await;
    let residents = [
        u32::try_from(pair.a.pid()).unwrap(),
        u32::try_from(pair.b.pid()).unwrap(),
    ];
    eprintln!(
        "[netlab-cpu-node] {}",
        json!({"residents": residents, "harness_pid": std::process::id()})
    );
    let mut load = BusyCpu::start(&residents)
        .await
        .expect("actual residents admit the checked CPU load");
    let (_, error) = super::echo(&pair.a).await;
    assert!(
        error.is_none(),
        "real residents answer under load: {error:?}"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    load.verify("real-resident-calibration").unwrap();
    drop(load);
    crate::lab::result(
        "cpu-node-admit",
        true,
        "two actual khor residents admitted on one CPU; real request answered; contenders advanced",
    );
}

#[tokio::test]
#[ignore = "separate CPU calibration; never alongside timed netlab"]
async fn failed_pinning_or_disallowed_cpu_cannot_admit_load() {
    let child = Reaped(Command::new("sleep").arg("10").spawn().unwrap());
    let before = Process::read(child.0.id()).unwrap();
    let cpu = current_allowed().unwrap()[0];
    let failed = pin("false", &[child.0.id()], cpu);
    assert!(failed.is_err());
    assert!(
        BusyCpu::start_on(&[child.0.id()], usize::MAX)
            .await
            .is_err()
    );
    let after = Process::read(child.0.id()).unwrap();
    assert_eq!(before.start, after.start);
    assert_eq!(before.threads, after.threads);
    crate::lab::result(
        "cpu-reject",
        true,
        "nonzero pin command and disallowed CPU rejected; owned child affinity unchanged",
    );
}
