//! Prints the TS face of zh.toml. The GUI build pipes this into its
//! source tree so both ends read one wording file and never drift.

fn main() {
    print!("{}", include_str!(concat!(env!("OUT_DIR"), "/catalog.ts")));
}
