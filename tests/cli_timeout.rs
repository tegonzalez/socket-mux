//! The global timeout option on the command line (design SM-D19; spelling PROPOSAL SM-D21).

mod common;

use common::bin;

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = bin().args(args).output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn help_lists_timeout_with_default_ten_seconds() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, Some(0));
    let line = stdout
        .lines()
        .find(|l| l.contains("--timeout"))
        .expect("--help lists --timeout");
    assert!(line.contains("10"), "default 10 s in: {line}");
}

#[test]
fn bad_timeout_values_are_usage_errors() {
    for args in [
        &["--timeout", "0", "rx"][..],
        &["--timeout", "-1", "rx"],
        &["--timeout", "x", "rx"],
        &["rx", "--timeout"],
        &["tx-pat", "--timeout", "0"],
    ] {
        let (code, _, stderr) = run(args);
        assert_eq!(code, Some(2), "{args:?}: stderr {stderr}");
        assert!(!stderr.is_empty(), "{args:?}: usage message on stderr");
    }
}
