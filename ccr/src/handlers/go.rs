use super::util;
use super::Handler;

pub struct GoHandler;

impl Handler for GoHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "build" | "install" | "vet" => filter_build(output),
            "test" => filter_test(output),
            "run" => filter_run(output),
            "mod" => filter_mod(output),
            _ => output.to_string(),
        }
    }
}

fn filter_build(output: &str) -> String {
    // Go build errors look like: "path/file.go:42:5: error message"
    let errors: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && (t.contains(": undefined")
                    || t.contains(": cannot")
                    || t.contains(": syntax error")
                    || t.contains(": declared and not used")
                    || t.contains(": imported and not used")
                    || t.contains(": too many")
                    || t.contains(": not enough")
                    || (t.contains(".go:") && t.contains(": ")))
        })
        .collect();

    if errors.is_empty() {
        if output.trim().is_empty() {
            return "[build OK]".to_string();
        }
        return output.to_string();
    }
    errors.join("\n")
}

fn filter_test(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0usize;

    for line in &lines {
        let t = line.trim();
        // FAIL package line
        if t.starts_with("FAIL") || t.starts_with("--- FAIL:") {
            out.push(line.to_string());
            in_failure = true;
            failure_lines = 0;
            continue;
        }
        // Panic
        if t.starts_with("panic:") || t.starts_with("goroutine ") {
            out.push(line.to_string());
            continue;
        }
        if in_failure {
            if failure_lines < 10 {
                out.push(line.to_string());
                failure_lines += 1;
            } else if failure_lines == 10 {
                out.push("[... truncated ...]".to_string());
                failure_lines += 1;
            }
            // --- PASS or blank line ends failure block
            if t.starts_with("--- PASS") || (t.is_empty() && failure_lines > 2) {
                in_failure = false;
            }
            continue;
        }
        // Summary: ok / FAIL with package + time
        if (t.starts_with("ok ") || t.starts_with("FAIL\t") || t.starts_with("FAIL "))
            && t.contains('\t')
        {
            out.push(line.to_string());
            continue;
        }
        // Error output
        if util::is_hard_keep(t) {
            out.push(line.to_string());
        }
        // Drop: --- PASS lines, "=== RUN" lines, "coverage:" lines
    }

    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_run(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= 50 {
        return output.to_string();
    }
    // Traceback / panic: keep from the panic line onward
    if let Some(pos) = output.find("goroutine 1 [running]:") {
        return output[pos..].to_string();
    }
    if let Some(pos) = output.find("panic:") {
        return output[pos..].to_string();
    }
    // Long output: BERT summarize
    let result = ccr_core::summarizer::summarize(output, 40);
    result.output
}

fn filter_mod(output: &str) -> String {
    // go mod tidy / download — keep warnings and errors only
    let important: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && (util::is_hard_keep(t)
                    || t.starts_with("go: ")
                    || t.contains("module")
                    || t.contains("version"))
        })
        .take(20)
        .collect();
    if important.is_empty() {
        "[go mod complete]".to_string()
    } else {
        important.join("\n")
    }
}
