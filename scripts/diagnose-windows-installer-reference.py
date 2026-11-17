"""Trace one original installer check with an earlier verified package, not product acceptance."""
import datetime,hashlib,json,os,platform,runpy,subprocess,sys,time
from pathlib import Path
assert os.name=="nt" and os.environ.get("GITHUB_ACTIONS")=="true"
report=Path(os.environ["REFERENCE_REPORT"]);report.mkdir(parents=True)
log=(report/"trace.jsonl").open("x",encoding="utf-8")
def emit(event,**fields):
 row=dict(at=datetime.datetime.now(datetime.timezone.utc).isoformat(),event=event,**fields);text=json.dumps(row);print(text,flush=True);log.write(text+"\n");log.flush()
run_original=subprocess.run;popen_original=subprocess.Popen

def traced_run(*args,**kwargs):
 command=args[0] if args else kwargs.get("args");start=time.monotonic();emit("run_started",command=str(command),timeout=kwargs.get("timeout"))
 try:
  result=run_original(*args,**kwargs);emit("run_finished",returncode=result.returncode,elapsed_seconds=time.monotonic()-start);return result
 except BaseException as error:
  emit("run_failed",error=repr(error),elapsed_seconds=time.monotonic()-start);raise

def traced_popen(*args,**kwargs):
 command=args[0] if args else kwargs.get("args");emit("popen_started",command=str(command));process=popen_original(*args,**kwargs);emit("popen_created",pid=process.pid);return process

package=Path("reference-product/khor-windows-x86_64.msi").resolve(strict=True)
emit("reference_started",control_source=os.environ["GITHUB_SHA"],product_source="81cc76c48b48b2900f37c70e0307a47df29dcaa1",checker_sha256=hashlib.sha256(Path("scripts/check-desktop-installer.py").read_bytes()).hexdigest(),package_sha256=hashlib.sha256(package.read_bytes()).hexdigest(),python=platform.python_version(),system=platform.platform(),original_deadlines_unchanged=True,replacements=0,product_acceptance=False)
sys.path.insert(0,str(Path("scripts").resolve()));subprocess.run=traced_run;subprocess.Popen=traced_popen
sys.argv=["scripts/check-desktop-installer.py","--package",str(package),"--output",str(report/"installation.json"),"--expected-source","81cc76c48b48b2900f37c70e0307a47df29dcaa1"]
try:
 runpy.run_path("scripts/check-desktop-installer.py",run_name="__main__");emit("reference_finished",success=True)
finally:
 subprocess.run=run_original;subprocess.Popen=popen_original;log.close()
