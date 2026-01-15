use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct TimesBuiltin;

impl super::Builtin for TimesBuiltin {
    fn name(&self) -> &'static str {
        "times"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Display accumulated user and system CPU times for the shell and its child processes"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        _shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        // POSIX times takes no arguments
        if cmd.args.len() > 1 {
            eprintln!("times: too many arguments");
            return 1;
        }

        // Get resource usage for the shell process and its children
        let (shell_user, shell_sys) = get_rusage(libc::RUSAGE_SELF);
        let (children_user, children_sys) = get_rusage(libc::RUSAGE_CHILDREN);

        // Format and output the times
        // Format: XmY.ZZs (e.g., 0m0.12s)
        let _ = writeln!(
            output_writer,
            "{} {}",
            format_time(shell_user),
            format_time(shell_sys)
        );
        let _ = writeln!(
            output_writer,
            "{} {}",
            format_time(children_user),
            format_time(children_sys)
        );

        0
    }
}

/// Get resource usage for either the current process or its children
fn get_rusage(usage_type: i32) -> (f64, f64) {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;

        unsafe {
            let mut usage = MaybeUninit::<libc::rusage>::uninit();
            if libc::getrusage(usage_type, usage.as_mut_ptr()) == 0 {
                let usage = usage.assume_init();
                let user_time = timeval_to_seconds(&usage.ru_utime);
                let sys_time = timeval_to_seconds(&usage.ru_stime);
                return (user_time, sys_time);
            }
        }
    }

    // Fallback for non-Unix systems or if getrusage fails
    (0.0, 0.0)
}

/// Convert a timeval struct to seconds as a floating-point number
#[cfg(unix)]
fn timeval_to_seconds(tv: &libc::timeval) -> f64 {
    tv.tv_sec as f64 + (tv.tv_usec as f64 / 1_000_000.0)
}

/// Format time in the POSIX format: XmY.ZZs
/// Examples: 0m0.12s, 1m23.45s, 10m5.67s
fn format_time(seconds: f64) -> String {
    let minutes = (seconds as i64) / 60;
    let remaining_seconds = seconds - (minutes as f64 * 60.0);
    format!("{}m{:.2}s", minutes, remaining_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;

    #[test]
    fn test_times_builtin_basic_execution() {
        let cmd = ShellCommand {
            args: vec!["times".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TimesBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 0);
        
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        
        // Should have exactly 2 lines
        assert_eq!(lines.len(), 2, "times output should have exactly 2 lines");
        
        // Each line should have 2 time values
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(parts.len(), 2, "each line should have 2 time values");
            
            // Each time value should match the format XmY.ZZs
            for part in parts {
                assert!(part.contains('m'), "time should contain 'm'");
                assert!(part.ends_with('s'), "time should end with 's'");
            }
        }
    }

    #[test]
    fn test_times_builtin_no_arguments() {
        let cmd = ShellCommand {
            args: vec!["times".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TimesBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_times_builtin_rejects_arguments() {
        let cmd = ShellCommand {
            args: vec!["times".to_string(), "arg1".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TimesBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 1);
        
        assert!(output.is_empty(), "no output should be written to stdout on error");
    }

    #[test]
    fn test_format_time() {
        // Test various time values
        assert_eq!(format_time(0.0), "0m0.00s");
        assert_eq!(format_time(0.12), "0m0.12s");
        assert_eq!(format_time(1.5), "0m1.50s");
        assert_eq!(format_time(59.99), "0m59.99s");
        assert_eq!(format_time(60.0), "1m0.00s");
        assert_eq!(format_time(83.45), "1m23.45s");
        assert_eq!(format_time(605.67), "10m5.67s");
        assert_eq!(format_time(3661.89), "61m1.89s");
    }

    #[test]
    fn test_times_output_format() {
        let cmd = ShellCommand {
            args: vec!["times".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        let builtin = TimesBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
        
        assert_eq!(exit_code, 0);
        
        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.trim().split('\n').collect();
        
        // Verify format of first line (shell times)
        let shell_times: Vec<&str> = lines[0].split_whitespace().collect();
        assert_eq!(shell_times.len(), 2);
        
        // Verify format of second line (children times)
        let children_times: Vec<&str> = lines[1].split_whitespace().collect();
        assert_eq!(children_times.len(), 2);
        
        // Verify all times match the pattern XmY.ZZs
        for time_str in shell_times.iter().chain(children_times.iter()) {
            // Should have format like "0m0.12s"
            let m_pos = time_str.find('m').expect("should contain 'm'");
            let s_pos = time_str.find('s').expect("should contain 's'");
            
            assert!(m_pos < s_pos, "m should come before s");
            assert!(s_pos == time_str.len() - 1, "s should be at the end");
            
            // Extract the seconds part and verify it has 2 decimal places
            let seconds_part = &time_str[m_pos + 1..s_pos];
            assert!(seconds_part.contains('.'), "seconds should have decimal point");
            
            let decimal_pos = seconds_part.find('.').unwrap();
            let decimal_places = seconds_part.len() - decimal_pos - 1;
            assert_eq!(decimal_places, 2, "should have exactly 2 decimal places");
        }
    }

    #[test]
    fn test_times_builtin_name() {
        let builtin = TimesBuiltin;
        assert_eq!(builtin.name(), "times");
        assert_eq!(builtin.names(), vec!["times"]);
    }

    #[test]
    fn test_times_builtin_description() {
        let builtin = TimesBuiltin;
        assert!(!builtin.description().is_empty());
        assert!(builtin.description().contains("CPU"));
    }
}
