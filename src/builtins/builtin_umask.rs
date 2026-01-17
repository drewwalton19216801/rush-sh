use std::io::Write;

use crate::parser::ShellCommand;
use crate::state::ShellState;

pub struct UmaskBuiltin;

impl super::Builtin for UmaskBuiltin {
    fn name(&self) -> &'static str {
        "umask"
    }

    fn names(&self) -> Vec<&'static str> {
        vec![self.name()]
    }

    fn description(&self) -> &'static str {
        "Set or display file mode creation mask"
    }

    fn run(
        &self,
        cmd: &ShellCommand,
        shell_state: &mut ShellState,
        output_writer: &mut dyn Write,
    ) -> i32 {
        let args = &cmd.args;

        // Parse options
        let mut symbolic_display = false;
        let mut arg_index = 1;

        while arg_index < args.len() && args[arg_index].starts_with('-') && args[arg_index] != "-" {
            match args[arg_index].as_str() {
                "-S" => symbolic_display = true,
                "--" => {
                    arg_index += 1;
                    break;
                }
                opt => {
                    let _ = writeln!(output_writer, "umask: invalid option: {}", opt);
                    return 1;
                }
            }
            arg_index += 1;
        }

        // Check for mask operand
        if arg_index < args.len() {
            // Check for extra operands
            if args.len() > arg_index + 1 {
                let _ = writeln!(output_writer, "umask: extra operand");
                return 1;
            }

            // Setting umask
            let mask_str = &args[arg_index];

            // Try parsing as octal first
            let new_mask = if mask_str.chars().all(|c| c.is_ascii_digit()) {
                match parse_octal_mask(mask_str) {
                    Ok(mask) => mask,
                    Err(e) => {
                        let _ = writeln!(output_writer, "umask: {}", e);
                        return 1;
                    }
                }
            } else {
                // Parse as symbolic
                match parse_symbolic_mask(mask_str, shell_state.umask) {
                    Ok(mask) => mask,
                    Err(e) => {
                        let _ = writeln!(output_writer, "umask: {}", e);
                        return 1;
                    }
                }
            };

            // Set the new umask
            shell_state.umask = new_mask;
            set_process_umask(new_mask);

            // Display in symbolic format if -S was specified
            if symbolic_display {
                display_umask_symbolic(new_mask, output_writer)
            } else {
                0
            }
        } else {
            // Displaying umask
            if symbolic_display {
                display_umask_symbolic(shell_state.umask, output_writer)
            } else {
                display_umask_numeric(shell_state.umask, output_writer)
            }
        }
    }
}

/// Display umask in numeric format (4-digit octal)
fn display_umask_numeric(umask: u32, output_writer: &mut dyn Write) -> i32 {
    if writeln!(output_writer, "{:04o}", umask).is_err() {
        return 1;
    }
    0
}

/// Display umask in symbolic format (u=rwx,g=rx,o=rx)
fn display_umask_symbolic(umask: u32, output_writer: &mut dyn Write) -> i32 {
    // Convert umask to allowed permissions (complement)
    let allowed = 0o777 & !umask;

    // Extract permission bits
    let user_perms = (allowed >> 6) & 0o7;
    let group_perms = (allowed >> 3) & 0o7;
    let other_perms = allowed & 0o7;

    // Build symbolic string
    let mut result = String::from("u=");
    if user_perms & 0o4 != 0 {
        result.push('r');
    }
    if user_perms & 0o2 != 0 {
        result.push('w');
    }
    if user_perms & 0o1 != 0 {
        result.push('x');
    }

    result.push_str(",g=");
    if group_perms & 0o4 != 0 {
        result.push('r');
    }
    if group_perms & 0o2 != 0 {
        result.push('w');
    }
    if group_perms & 0o1 != 0 {
        result.push('x');
    }

    result.push_str(",o=");
    if other_perms & 0o4 != 0 {
        result.push('r');
    }
    if other_perms & 0o2 != 0 {
        result.push('w');
    }
    if other_perms & 0o1 != 0 {
        result.push('x');
    }

    if writeln!(output_writer, "{}", result).is_err() {
        return 1;
    }
    0
}

/// Parse octal mask string (e.g., "022", "0022")
fn parse_octal_mask(mask_str: &str) -> Result<u32, String> {
    // Validate: only octal digits
    if !mask_str.chars().all(|c| c.is_ascii_digit() && c <= '7') {
        return Err(format!("invalid octal number: {}", mask_str));
    }

    // Handle empty string after validation
    if mask_str.is_empty() {
        return Err(format!("invalid octal number: {}", mask_str));
    }

    // Remove optional leading zero (but keep at least one digit)
    let mask_str = if mask_str.len() > 1 && mask_str.starts_with('0') {
        &mask_str[1..]
    } else {
        mask_str
    };

    // Parse as octal
    match u32::from_str_radix(mask_str, 8) {
        Ok(mask) if mask <= 0o777 => Ok(mask),
        _ => Err(format!("invalid octal number: {}", mask_str)),
    }
}

/// Parse symbolic mask string (e.g., "u=rwx,g=rx,o=")
fn parse_symbolic_mask(symbolic_str: &str, current_umask: u32) -> Result<u32, String> {
    // Start with current allowed permissions (complement of umask)
    let mut allowed = 0o777 & !current_umask;

    // Split by comma for multiple clauses
    for clause in symbolic_str.split(',') {
        if clause.is_empty() {
            continue;
        }

        // Parse clause: [ugoa...][+-=][rwxXst...]
        let (who, op, perms) = parse_symbolic_clause(clause)?;

        // Apply operation to allowed permissions
        allowed = apply_symbolic_operation(allowed, who, op, perms)?;
    }

    // Convert allowed permissions back to umask (complement)
    Ok(0o777 & !allowed)
}

/// Parse a single symbolic clause (e.g., "u=rwx", "g-w", "o+r")
fn parse_symbolic_clause(clause: &str) -> Result<(u32, char, u32), String> {
    // Parse who (ugoa)
    let mut who = 0u32;
    let mut i = 0;
    for c in clause.chars() {
        match c {
            'u' => who |= 0o700,
            'g' => who |= 0o070,
            'o' => who |= 0o007,
            'a' => who |= 0o777,
            '+' | '-' | '=' => break,
            _ => return Err(format!("invalid symbolic mode: {}", clause)),
        }
        i += 1;
    }

    // If no who specified, default to 'a' (all)
    if who == 0 {
        who = 0o777;
    }

    // Parse operator
    let op = clause.chars().nth(i).ok_or_else(|| {
        format!("invalid symbolic mode: {}", clause)
    })?;

    if !matches!(op, '+' | '-' | '=') {
        return Err(format!("invalid symbolic mode: {}", clause));
    }

    // Parse permissions
    let perm_str = &clause[i + 1..];
    let mut perms = 0u32;
    for c in perm_str.chars() {
        match c {
            'r' => perms |= 0o444,
            'w' => perms |= 0o222,
            'x' => perms |= 0o111,
            _ => return Err(format!("invalid symbolic mode: {}", clause)),
        }
    }

    // Mask permissions by who
    perms &= who;

    Ok((who, op, perms))
}

/// Apply symbolic operation to allowed permissions
fn apply_symbolic_operation(allowed: u32, who: u32, op: char, perms: u32) -> Result<u32, String> {
    match op {
        '+' => Ok(allowed | perms),
        '-' => Ok(allowed & !perms),
        '=' => Ok((allowed & !who) | perms),
        _ => Err(format!("invalid operator: {}", op)),
    }
}

/// Set the process umask
fn set_process_umask(mask: u32) {
    unsafe {
        libc::umask(mask as libc::mode_t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::Builtin;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify the process umask (global state)
    static UMASK_LOCK: Mutex<()> = Mutex::new(());
    
    // Mutex to serialize tests that modify environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper function to run tests with umask lock and automatic restoration
    ///
    /// This function:
    /// 1. Acquires the UMASK_LOCK mutex
    /// 2. Saves the current process umask
    /// 3. Runs the provided closure
    /// 4. Restores the saved umask (even on panic)
    /// 5. Releases the lock automatically
    fn with_umask_lock<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _lock = UMASK_LOCK.lock().unwrap();
        
        // Save current umask by setting it to 0 and capturing the old value
        let saved_umask = unsafe { libc::umask(0) };
        
        // Restore the umask immediately so we have the original value
        unsafe { libc::umask(saved_umask); }
        
        // Use a guard to ensure umask is restored even on panic
        struct UmaskGuard(libc::mode_t);
        impl Drop for UmaskGuard {
            fn drop(&mut self) {
                unsafe {
                    libc::umask(self.0);
                }
            }
        }
        let _guard = UmaskGuard(saved_umask);
        
        // Run the test closure
        f()
    }

    #[test]
    fn test_umask_display_numeric() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.umask = 0o022;

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str.trim(), "0022");
    }

    #[test]
    fn test_umask_display_symbolic() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string(), "-S".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();
        shell_state.umask = 0o022;

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str.trim(), "u=rwx,g=rx,o=rx");
    }

    #[test]
    fn test_umask_set_octal() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "027".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o027);
        });
    }

    #[test]
    fn test_umask_set_octal_with_leading_zero() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "0027".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o027);
        });
    }

    #[test]
    fn test_umask_set_symbolic() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "u=rwx,g=rx,o=".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o027);
        });
    }

    #[test]
    fn test_umask_invalid_octal() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string(), "999".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid octal number"));
    }

    #[test]
    fn test_umask_invalid_option() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string(), "-X".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid option"));
    }

    #[test]
    fn test_umask_symbolic_with_s_flag() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "-S".to_string(), "022".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o022);
            let output_str = String::from_utf8(output).unwrap();
            assert_eq!(output_str.trim(), "u=rwx,g=rx,o=rx");
        });
    }

    #[test]
    fn test_umask_various_values() {
        let _lock = ENV_LOCK.lock().unwrap();
        let test_cases = vec![
            (0o000, "0000", "u=rwx,g=rwx,o=rwx"),
            (0o022, "0022", "u=rwx,g=rx,o=rx"),
            (0o027, "0027", "u=rwx,g=rx,o="),
            (0o077, "0077", "u=rwx,g=,o="),
            (0o002, "0002", "u=rwx,g=rwx,o=rx"),
            (0o777, "0777", "u=,g=,o="),
        ];

        for (mask, expected_numeric, expected_symbolic) in test_cases {
            let mut shell_state = ShellState::new();
            shell_state.umask = mask;

            // Test numeric display
            let cmd = ShellCommand {
                args: vec!["umask".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
            assert_eq!(exit_code, 0);
            let output_str = String::from_utf8(output).unwrap();
            assert_eq!(output_str.trim(), expected_numeric);

            // Test symbolic display
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "-S".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);
            assert_eq!(exit_code, 0);
            let output_str = String::from_utf8(output).unwrap();
            assert_eq!(output_str.trim(), expected_symbolic);
        }
    }

    #[test]
    fn test_parse_octal_mask() {
        assert_eq!(parse_octal_mask("022").unwrap(), 0o022);
        assert_eq!(parse_octal_mask("0022").unwrap(), 0o022);
        assert_eq!(parse_octal_mask("777").unwrap(), 0o777);
        assert_eq!(parse_octal_mask("0").unwrap(), 0o000);
        assert_eq!(parse_octal_mask("2").unwrap(), 0o002);
        assert!(parse_octal_mask("999").is_err());
        assert!(parse_octal_mask("888").is_err());
        assert!(parse_octal_mask("abc").is_err());
    }

    #[test]
    fn test_parse_symbolic_mask() {
        // Test basic symbolic modes
        assert_eq!(parse_symbolic_mask("u=rwx,g=rx,o=", 0o022).unwrap(), 0o027);
        assert_eq!(parse_symbolic_mask("u=rwx,g=rwx,o=rwx", 0o022).unwrap(), 0o000);
        assert_eq!(parse_symbolic_mask("a=", 0o022).unwrap(), 0o777);

        // Test with current umask
        assert_eq!(parse_symbolic_mask("u+w", 0o022).unwrap(), 0o022); // No change
        assert_eq!(parse_symbolic_mask("g-w", 0o022).unwrap(), 0o022); // Already masked
    }

    #[test]
    fn test_umask_builtin_name() {
        let builtin = UmaskBuiltin;
        assert_eq!(builtin.name(), "umask");
        assert_eq!(builtin.names(), vec!["umask"]);
    }

    #[test]
    fn test_umask_builtin_description() {
        let builtin = UmaskBuiltin;
        assert!(!builtin.description().is_empty());
        assert!(builtin.description().contains("mask"));
    }

    #[test]
    fn test_umask_set_zero() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "0".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o000);
        });
    }

    #[test]
    fn test_umask_set_max() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "777".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o777);
        });
    }

    #[test]
    fn test_umask_invalid_octal_too_large() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string(), "1000".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid octal number"));
    }

    #[test]
    fn test_umask_symbolic_all_permissions() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "a=rwx".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o000);
        });
    }

    #[test]
    fn test_umask_symbolic_no_permissions() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "a=".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o777);
        });
    }

    #[test]
    fn test_umask_symbolic_add_permission() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "g+w".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();
            shell_state.umask = 0o027; // g has rx, not w

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o007); // g now has rwx
        });
    }

    #[test]
    fn test_umask_symbolic_remove_permission() {
        with_umask_lock(|| {
            let _env_lock = ENV_LOCK.lock().unwrap();
            let cmd = ShellCommand {
                args: vec!["umask".to_string(), "g-r".to_string()],
                redirections: Vec::new(),
                compound: None,
            };
            let mut shell_state = ShellState::new();
            shell_state.umask = 0o022; // g has rx

            let builtin = UmaskBuiltin;
            let mut output = Vec::new();
            let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

            assert_eq!(exit_code, 0);
            assert_eq!(shell_state.umask, 0o062); // g now has only x
        });
    }

    #[test]
    fn test_umask_invalid_symbolic_mode() {
        let _lock = ENV_LOCK.lock().unwrap();
        let cmd = ShellCommand {
            args: vec!["umask".to_string(), "u=invalid".to_string()],
            redirections: Vec::new(),
            compound: None,
        };
        let mut shell_state = ShellState::new();

        let builtin = UmaskBuiltin;
        let mut output = Vec::new();
        let exit_code = builtin.run(&cmd, &mut shell_state, &mut output);

        assert_eq!(exit_code, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("invalid symbolic mode"));
    }

    #[test]
    fn test_display_umask_numeric_format() {
        let mut output = Vec::new();
        let exit_code = display_umask_numeric(0o022, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str.trim(), "0022");
    }

    #[test]
    fn test_display_umask_symbolic_format() {
        let mut output = Vec::new();
        let exit_code = display_umask_symbolic(0o022, &mut output);
        assert_eq!(exit_code, 0);
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str.trim(), "u=rwx,g=rx,o=rx");
    }

    #[test]
    fn test_parse_symbolic_clause_user() {
        let (who, op, perms) = parse_symbolic_clause("u=rwx").unwrap();
        assert_eq!(who, 0o700);
        assert_eq!(op, '=');
        assert_eq!(perms, 0o700);
    }

    #[test]
    fn test_parse_symbolic_clause_group() {
        let (who, op, perms) = parse_symbolic_clause("g+w").unwrap();
        assert_eq!(who, 0o070);
        assert_eq!(op, '+');
        assert_eq!(perms, 0o020);
    }

    #[test]
    fn test_parse_symbolic_clause_other() {
        let (who, op, perms) = parse_symbolic_clause("o-x").unwrap();
        assert_eq!(who, 0o007);
        assert_eq!(op, '-');
        assert_eq!(perms, 0o001);
    }

    #[test]
    fn test_parse_symbolic_clause_all() {
        let (who, op, perms) = parse_symbolic_clause("a=rx").unwrap();
        assert_eq!(who, 0o777);
        assert_eq!(op, '=');
        assert_eq!(perms, 0o555);
    }

    #[test]
    fn test_apply_symbolic_operation_add() {
        let result = apply_symbolic_operation(0o755, 0o070, '+', 0o020).unwrap();
        assert_eq!(result, 0o775);
    }

    #[test]
    fn test_apply_symbolic_operation_remove() {
        let result = apply_symbolic_operation(0o755, 0o070, '-', 0o040).unwrap();
        assert_eq!(result, 0o715);
    }

    #[test]
    fn test_apply_symbolic_operation_set() {
        let result = apply_symbolic_operation(0o755, 0o070, '=', 0o020).unwrap();
        assert_eq!(result, 0o725);
    }
}

