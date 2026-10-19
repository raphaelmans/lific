use std::process::Command;

#[test]
fn personal_help_has_no_remote_credentials() {
    let secret = "lific-help-must-not-print-this-test-key";
    for args in [vec!["--help"], vec!["doctor", "--help"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_lific"))
            .env("LIFIC_API_KEY", secret)
            .env("LIFIC_TOKEN", secret)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stdout.contains("LIFIC_API_KEY"));
        assert!(!stdout.contains(secret));
        assert!(!stderr.contains(secret));
    }
}
