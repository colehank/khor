// What a block's output reads as (`src/lib/term-peek.ts`, roadmap 3.4):
// real outputs that are data, and real outputs that are not.
import { peekOf } from "../src/lib/term-peek.ts";

const tables = {
  // Real, from this Mac (user and paths changed): a CJK start time, a
  // start date with a space in it, a user name wider than its column.
  "ps aux": `USER               PID  %CPU %MEM      VSZ    RSS   TT  STAT STARTED      TIME COMMAND
someone           2498  65.6  0.4 435655392  61232   ??  S     3:16上午  16:45.33 /opt/khor/bridge --port 1474
_windowserver      418  42.7  0.4 436418400  69648   ??  Ss   29 8月26 15956:38.98 /System/Library/WindowServer -daemon`,
  "docker ps": `CONTAINER ID   IMAGE          COMMAND                  CREATED        STATUS        PORTS     NAMES
3f2a1b9c8d7e   nginx:1.27     "/docker-entrypoint.…"   2 hours ago    Up 2 hours    80/tcp    web
9a8b7c6d5e4f   redis:7        "docker-entrypoint.s…"   3 days ago     Up 3 days     6379/tcp  cache`,
  "df -h": `Filesystem        Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s1s1   460Gi    10Gi   201Gi     5%    404k  2.1G    0%   /
/dev/disk3s6     460Gi   2.0Gi   201Gi     1%       2  2.1G    0%   /System/Volumes/VM`,
  "kubectl get pods": `NAME                     READY   STATUS    RESTARTS   AGE
web-5d8c7b9f6-abcde      1/1     Running   0          3d
worker-7f6e5d4c3-fghij   0/1     Pending   2          5m`,
  csv: `name,size,kind
a.txt,12,file
"b, c",40,dir`,
  tsv: "name\tsize\na.txt\t12\nsrc\t40",
};
const json = {
  object: `{"name": "khor", "deps": [1, 2, {"x": null}]}`,
  "JSON Lines": `{"level":"info","msg":"up"}\n{"level":"warn","msg":"slow"}`,
};
const text = {
  "ls (a grid of names)": `Cargo.lock     README.md      crates         node_modules
Cargo.toml     apps           docs           scripts
LICENSE        audit          target         tests`,
  "git status": `On branch main
Changes not staged for commit:
        modified:   src/x.ts`,
  "a sentence with commas": `one, two, three
four, five, six
seven, eight, nine`,
  "a traceback": `Traceback (most recent call last):
  File "bad.py", line 1, in <module>
NameError: name 'x' is not defined`,
  "one number": "42",
  "a JSON string": `"just a string"`,
};

let bad = 0;
const check = (name, ok, why) => {
  if (!ok) {
    bad++;
    console.log(`peek gate: ${name}: ${why}`);
  }
};
for (const [name, out] of Object.entries(tables)) {
  const p = peekOf(out);
  check(name, p?.kind === "table" && p.rows.every((r) => r.length === p.head.length), `want a table, got ${JSON.stringify(p)?.slice(0, 160)}`);
}
// ps itself lets a long TIME push past its column on the second row, so
// no blank column runs between STARTED, TIME and COMMAND: they are one
// column, as they are on the screen. The columns before are measured in
// cells (上午 is four), and the last keeps its spaces.
const ps = peekOf(tables["ps aux"]);
check("ps aux", ps?.rows[0][1] === "2498" && ps?.rows[1][7] === "Ss", "the columns before the wide characters");
check("ps aux", ps?.rows[0].at(-1) === "3:16上午  16:45.33 /opt/khor/bridge --port 1474", "the last column keeps its spaces");
const ascii = peekOf(`USER               PID  %CPU %MEM      VSZ    RSS   TT  STAT STARTED      TIME COMMAND
someone           2498  65.6  0.4  4356553  61232   ??  S    3:16AM   16:45.33 /opt/khor/bridge --port 1474
_windowserver      418  42.7  0.4  4364184  69648   ??  Ss   Thu09AM  56:38.98 /System/Library/WindowServer -daemon`);
check("ps aux, aligned", ascii?.head.at(-1) === "COMMAND" && ascii?.rows[0].at(-1) === "/opt/khor/bridge --port 1474", "aligned, every column apart");
check("docker ps", peekOf(tables["docker ps"])?.head[0] === "CONTAINER ID", "a two-word header is one column");
check("csv", peekOf(tables.csv)?.rows[1][0] === "b, c", "a quoted comma stays in its cell");
for (const [name, out] of Object.entries(json)) check(name, peekOf(out)?.kind === "json", "want JSON");
for (const [name, out] of Object.entries(text)) check(name, peekOf(out) === null, `want nothing, got ${JSON.stringify(peekOf(out))?.slice(0, 160)}`);
console.log(bad ? `peek gate: ${bad} wrong` : `peek gate: ${Object.keys(tables).length + Object.keys(json).length} read as data, ${Object.keys(text).length} as text`);
process.exit(bad ? 1 : 0);
