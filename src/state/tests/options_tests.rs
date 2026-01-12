//! Tests for shell options management

use crate::state::ShellOptions;

#[test]
fn test_shell_options_default() {
    let options = ShellOptions::default();
    assert!(!options.errexit);
    assert!(!options.nounset);
    assert!(!options.xtrace);
    assert!(!options.verbose);
    assert!(!options.noexec);
    assert!(!options.noglob);
    assert!(!options.noclobber);
    assert!(!options.allexport);
    assert!(!options.notify);
    assert!(!options.ignoreeof);
    assert!(!options.monitor);
}

#[test]
fn test_shell_options_get_by_short_name() {
    let mut options = ShellOptions::default();
    options.errexit = true;
    options.nounset = true;

    assert_eq!(options.get_by_short_name('e'), Some(true));
    assert_eq!(options.get_by_short_name('u'), Some(true));
    assert_eq!(options.get_by_short_name('x'), Some(false));
    assert_eq!(options.get_by_short_name('Z'), None);
}

#[test]
fn test_shell_options_set_by_short_name() {
    let mut options = ShellOptions::default();

    assert!(options.set_by_short_name('e', true).is_ok());
    assert!(options.errexit);

    assert!(options.set_by_short_name('u', true).is_ok());
    assert!(options.nounset);

    assert!(options.set_by_short_name('x', true).is_ok());
    assert!(options.xtrace);

    assert!(options.set_by_short_name('e', false).is_ok());
    assert!(!options.errexit);

    // Invalid option
    assert!(options.set_by_short_name('Z', true).is_err());
}

#[test]
fn test_shell_options_get_by_long_name() {
    let mut options = ShellOptions::default();
    options.errexit = true;
    options.nounset = true;

    assert_eq!(options.get_by_long_name("errexit"), Some(true));
    assert_eq!(options.get_by_long_name("nounset"), Some(true));
    assert_eq!(options.get_by_long_name("xtrace"), Some(false));
    assert_eq!(options.get_by_long_name("invalid"), None);
}

#[test]
fn test_shell_options_set_by_long_name() {
    let mut options = ShellOptions::default();

    assert!(options.set_by_long_name("errexit", true).is_ok());
    assert!(options.errexit);

    assert!(options.set_by_long_name("nounset", true).is_ok());
    assert!(options.nounset);

    assert!(options.set_by_long_name("xtrace", true).is_ok());
    assert!(options.xtrace);

    assert!(options.set_by_long_name("errexit", false).is_ok());
    assert!(!options.errexit);

    // Invalid option
    assert!(options.set_by_long_name("invalid", true).is_err());
}

#[test]
fn test_shell_options_all_short_options() {
    let mut options = ShellOptions::default();

    // Test all valid short options
    let short_opts = vec!['e', 'u', 'x', 'v', 'n', 'f', 'C', 'a', 'b', 'm'];
    for opt in short_opts {
        assert!(options.set_by_short_name(opt, true).is_ok());
        assert_eq!(options.get_by_short_name(opt), Some(true));
        assert!(options.set_by_short_name(opt, false).is_ok());
        assert_eq!(options.get_by_short_name(opt), Some(false));
    }
}

#[test]
fn test_shell_options_all_long_options() {
    let mut options = ShellOptions::default();

    // Test all valid long options
    let long_opts = vec![
        "errexit",
        "nounset",
        "xtrace",
        "verbose",
        "noexec",
        "noglob",
        "noclobber",
        "allexport",
        "notify",
        "ignoreeof",
        "monitor",
    ];
    for opt in long_opts {
        assert!(options.set_by_long_name(opt, true).is_ok());
        assert_eq!(options.get_by_long_name(opt), Some(true));
        assert!(options.set_by_long_name(opt, false).is_ok());
        assert_eq!(options.get_by_long_name(opt), Some(false));
    }
}

#[test]
fn test_shell_options_get_all_options() {
    let mut options = ShellOptions::default();
    options.errexit = true;
    options.xtrace = true;

    let all_options = options.get_all_options();

    // Should have 11 options
    assert_eq!(all_options.len(), 11);

    // Find errexit and verify it's on
    let errexit_opt = all_options.iter().find(|(name, _, _)| *name == "errexit");
    assert!(errexit_opt.is_some());
    assert_eq!(errexit_opt.unwrap().2, true);

    // Find xtrace and verify it's on
    let xtrace_opt = all_options.iter().find(|(name, _, _)| *name == "xtrace");
    assert!(xtrace_opt.is_some());
    assert_eq!(xtrace_opt.unwrap().2, true);

    // Find nounset and verify it's off
    let nounset_opt = all_options.iter().find(|(name, _, _)| *name == "nounset");
    assert!(nounset_opt.is_some());
    assert_eq!(nounset_opt.unwrap().2, false);
}

#[test]
fn test_shell_state_has_options() {
    use crate::state::ShellState;
    let state = ShellState::new();
    assert!(!state.options.errexit);
    assert!(!state.options.nounset);
    assert!(!state.options.xtrace);
}

#[test]
fn test_shell_state_options_modification() {
    use crate::state::ShellState;
    let mut state = ShellState::new();

    state.options.errexit = true;
    assert!(state.options.errexit);

    state.options.set_by_short_name('u', true).unwrap();
    assert!(state.options.nounset);

    state.options.set_by_long_name("xtrace", true).unwrap();
    assert!(state.options.xtrace);
}

#[test]
fn test_shell_options_error_messages() {
    let mut options = ShellOptions::default();

    let result = options.set_by_short_name('Z', true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid option: -Z"));

    let result = options.set_by_long_name("invalid_option", true);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Invalid option: invalid_option")
    );
}

#[test]
fn test_shell_options_case_sensitivity() {
    let mut options = ShellOptions::default();

    // 'C' is valid (noclobber), 'c' is not
    assert!(options.set_by_short_name('C', true).is_ok());
    assert!(options.noclobber);
    assert!(options.set_by_short_name('c', true).is_err());
}