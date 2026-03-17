use super::util;
use super::Handler;

pub struct MavenHandler;

impl Handler for MavenHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();

        let build_success = lines.iter().any(|l| l.contains("BUILD SUCCESS"));
        let build_failure = lines.iter().any(|l| l.contains("BUILD FAILURE"));

        let mut out: Vec<String> = Vec::new();
        let mut in_error_block = false;
        let mut in_test_failure = false;
        let mut test_failure_lines = 0usize;

        for line in &lines {
            let t = line.trim();

            // Drop pure noise lines
            if is_noise(t) {
                continue;
            }

            // Error/failure blocks start with [ERROR]
            if t.starts_with("[ERROR]") {
                in_error_block = true;
                out.push(line.to_string());
                continue;
            }

            // Test failure blocks in surefire output
            if t.contains("Tests run:") && t.contains("Failures:") {
                out.push(line.to_string());
                continue;
            }
            if t.starts_with("FAILED") || t.starts_with("<<<") {
                in_test_failure = true;
                test_failure_lines = 0;
                out.push(line.to_string());
                continue;
            }
            if in_test_failure {
                if test_failure_lines < 15 {
                    out.push(line.to_string());
                    test_failure_lines += 1;
                } else if test_failure_lines == 15 {
                    out.push("[... truncated ...]".to_string());
                    test_failure_lines += 1;
                }
                if t.starts_with(">>>") || t.is_empty() {
                    in_test_failure = false;
                }
                continue;
            }

            // Keep [WARNING] lines
            if t.starts_with("[WARNING]") {
                out.push(line.to_string());
                continue;
            }

            if in_error_block && !t.starts_with("[") {
                // Continuation of error block
                out.push(line.to_string());
                continue;
            }
            in_error_block = false;

            // Keep reactor summary lines
            if t.starts_with("[INFO] ---")
                || t.starts_with("[INFO] BUILD")
                || t.starts_with("[INFO] Total time")
                || t.starts_with("[INFO] Reactor Summary")
                || t.starts_with("[INFO]  * ")
                || (t.starts_with("[INFO]") && t.contains("SUCCESS"))
                || (t.starts_with("[INFO]") && t.contains("FAILURE"))
            {
                out.push(line.to_string());
            }
        }

        if out.is_empty() {
            if build_success {
                return "[BUILD SUCCESS]".to_string();
            }
            return output.to_string();
        }

        // Append final result line if not already present
        if build_success && !out.iter().any(|l| l.contains("BUILD SUCCESS")) {
            out.push("[INFO] BUILD SUCCESS".to_string());
        } else if build_failure && !out.iter().any(|l| l.contains("BUILD FAILURE")) {
            out.push("[INFO] BUILD FAILURE".to_string());
        }

        out.join("\n")
    }
}

fn is_noise(t: &str) -> bool {
    if t.is_empty() {
        return false; // keep blank lines as separators
    }
    // Pure progress/download lines
    t.starts_with("[INFO] Downloading")
        || t.starts_with("[INFO] Downloaded")
        || t.starts_with("[INFO] Progress")
        || (t.starts_with("[INFO]") && t.contains("Compiling"))
        || t == "[INFO]"
        || (t.starts_with("[INFO] ---") && !t.contains("maven") && t.len() > 60
            && t.chars().filter(|&c| c == '-').count() > 20)
}

// Gradle produces different output — reuse same handler struct, same filtering logic applies.
pub struct GradleHandler;

impl Handler for GradleHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let mut out: Vec<String> = Vec::new();
        let mut in_failure = false;
        let mut failure_lines = 0usize;

        let build_success = lines.iter().any(|l| l.contains("BUILD SUCCESSFUL"));
        let build_failed = lines.iter().any(|l| l.contains("BUILD FAILED"));

        for line in &lines {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }

            // Task headers: "> Task :foo:bar"
            if t.starts_with("> Task") {
                if t.contains("FAILED") || t.contains("UP-TO-DATE") {
                    out.push(line.to_string());
                }
                continue;
            }

            // Compilation/test errors
            if t.starts_with("e: ") || t.starts_with("w: ") {
                // Kotlin compiler errors/warnings
                out.push(line.to_string());
                continue;
            }
            if t.contains(": error:") || t.contains(": warning:") {
                out.push(line.to_string());
                continue;
            }

            // Failure blocks
            if t.starts_with("FAILURE:") || t.starts_with("* What went wrong:") || t.starts_with("* Try:") {
                in_failure = true;
                failure_lines = 0;
                out.push(line.to_string());
                continue;
            }
            if in_failure {
                if failure_lines < 10 {
                    out.push(line.to_string());
                    failure_lines += 1;
                }
                if t.is_empty() || t.starts_with("* ") {
                    in_failure = false;
                }
                continue;
            }

            // Test failure lines
            if t.starts_with("FAILED") || util::is_hard_keep(t) {
                out.push(line.to_string());
                continue;
            }

            // Summary
            if t.starts_with("BUILD") || t.contains(" tests were") || t.contains("passed") {
                out.push(line.to_string());
            }
        }

        if out.is_empty() {
            if build_success {
                return "[BUILD SUCCESSFUL]".to_string();
            }
            if build_failed {
                return "[BUILD FAILED]".to_string();
            }
            return output.to_string();
        }

        if build_success && !out.iter().any(|l| l.contains("BUILD SUCCESSFUL")) {
            out.push("[BUILD SUCCESSFUL]".to_string());
        }

        out.join("\n")
    }
}
