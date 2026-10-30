//! The global timeout option on the command line (design SM-D19; spelling SM-D21).

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
    assert!(stdout.contains("--lanes N") && stdout.contains("default 1"));
    assert!(stdout.contains("--datagram-size BYTES") && stdout.contains("adapter MTU - 48"));
    assert!(stdout.contains("1452 if the MTU is unknown"));
    assert!(stdout.contains("--rx-memory BYTES") && stdout.contains("67108864"));
}

#[test]
fn bad_timeout_values_are_usage_errors() {
    for args in [
        &["--timeout", "0", "rx"][..],
        &["--timeout", "-1", "rx"],
        &["--timeout", "x", "rx"],
        &["rx", "--timeout"],
        &["tx-pat", "--timeout", "0"],
        &["rx", "--datagram-size", "24"],
        &["tx", "--datagram-size", "65508"],
        &["rx", "--rx-memory", "24"],
        &["rx", "--rx-memory"],
    ] {
        let (code, _, stderr) = run(args);
        assert_eq!(code, Some(2), "{args:?}: stderr {stderr}");
        assert!(!stderr.is_empty(), "{args:?}: usage message on stderr");
    }
    #[cfg(feature = "sequence-8")]
    assert_eq!(
        run(&["rx", "--datagram-size", "25", "--rx-memory", "3200"]).0,
        Some(2)
    );
    #[cfg(feature = "sequence-16")]
    assert_eq!(
        run(&["rx", "--datagram-size", "25", "--rx-memory", "819200"]).0,
        Some(2)
    );
}
