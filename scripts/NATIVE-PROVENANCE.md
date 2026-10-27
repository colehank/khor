# Native build provenance

`native_provenance.py` prepares and verifies a build; it never launches Cargo
or native tests. Keep Cargo in the existing guarded phase (sole shared target,
resource floor, deadlines, observed process groups, source/cleanup checks and
dual evidence). `run-node-acceptance.py` retains its original process controls
and requires the verified Node receipt, plus the normal CLI receipt when
`KHOR_TEST_MCP_EXECUTABLE` supplies the real worker. Quality CI wires both.

Use the existing build command and metadata captured from the current tree:

```sh
cargo metadata --locked --offline --format-version 1 > "$EVIDENCE/metadata.json"
python3 scripts/native_provenance.py prepare \
  --metadata "$EVIDENCE/metadata.json" --target-dir "$CARGO_TARGET_DIR" \
  --package khor-node --name khor_node --kind lib --test \
  --tool "$(rustup which cargo)" --tool "$(rustup which rustc)" \
  --prior "$EVIDENCE/cli-provenance.json" --apply-refresh \
  --output "$EVIDENCE/node-prepared.json"
# Run the original guarded cargo test --lib --no-run --message-format=json.
python3 scripts/native_provenance.py verify \
  --prepared "$EVIDENCE/node-prepared.json" \
  --artifacts "$EVIDENCE/node-artifacts.json" --output "$EVIDENCE/node-provenance.json"
```

Omit `--apply-refresh` for a read-only plan. Applying it requires ownership of
the Cargo slot and frozen source. Only unproven first-party lib/build/requested
target entries are touched; no cache files, third-party entries or fingerprints
are deleted. A changed dependency invalidates its first-party dependents too.
The complete tracked/nonignored source roster catches added modules even when
an old depfile omits them. Configuration, actual tool binaries, build variants,
environment hashes and embedded workspace roots participate in admission.
Root relocation is conservatively unproven, including otherwise identical
sources: this version makes no general relocation-equivalence claim.

For `fresh=true`, the actual files must match a previous verified producer with
the same source/build key and variant. Normal executable proof uses the unique
content/size/mtime-matched hashed executable and its rustc dep-info, never the
top-level summary `.d`. Identity and content are recorded separately; a known
Cargo publication may change inode without changing bytes. Cache retirement
remains the caller's existing exact-identity operation.

Build-script JSON events alone are not execution evidence. Generated outputs
need matching previous input/output content or a current invocation marker and
current output files, alongside the verified build-script artifact. Web dist
and an optional agent bundle are captured as actual extra build inputs. This
proves Rust's input bytes, not that an earlier frontend/bundle build came from
current source; final App acceptance still needs that producer's own receipt.
Cargo's third-party cache remains trusted under the recorded lock/config and
vendored-source stamps; these receipts govern first-party artifacts.

Pure Python tests use synthetic Cargo reports and private temporary files.
They do not establish actual compilation, native suite success, or the pending
same-target A → B → same-B → A experiment. Keep compile, provenance/setup,
native assertions, guard failure and cleanup failure distinct in the caller's
run receipt. Preserve prior evidence instead of overwriting it.
