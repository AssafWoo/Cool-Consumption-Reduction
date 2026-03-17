use super::Handler;

pub struct PipHandler;

impl Handler for PipHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let cmd = args.get(0).map(|s| s.as_str()).unwrap_or("");
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

        match subcmd {
            "freeze" | "list" => return output.to_string(),
            "install" | "add" => {
                if cmd == "uv" {
                    return filter_uv_install(output);
                }
                return filter_pip_install(output);
            }
            _ => {}
        }

        // Default: keep only final non-empty line
        output
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or(output)
            .to_string()
    }
}

fn filter_pip_install(output: &str) -> String {
    let mut warnings: Vec<String> = Vec::new();
    let mut installed = 0usize;

    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("Successfully installed") {
            installed += t
                .trim_start_matches("Successfully installed")
                .split_whitespace()
                .count();
        } else if t.to_uppercase().starts_with("WARNING") || t.to_uppercase().starts_with("ERROR") {
            warnings.push(line.to_string());
        }
    }

    let mut out: Vec<String> = warnings;
    if installed > 0 {
        out.push(format!("[pip install complete — {} packages]", installed));
    } else {
        let summary: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.contains("already satisfied")
                    || t.contains("Requirement already")
                    || t.to_uppercase().starts_with("ERROR")
            })
            .take(5)
            .collect();
        if !summary.is_empty() {
            out.extend(summary.iter().map(|l| l.to_string()));
        } else {
            return output.to_string();
        }
    }
    out.join("\n")
}

fn filter_uv_install(output: &str) -> String {
    // uv outputs: "Resolved N packages", "Prepared N packages", "Installed N packages", "Audited N packages"
    let mut warnings: Vec<String> = Vec::new();
    let mut installed = 0usize;
    let mut resolved = 0usize;

    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("Installed ") && t.contains("package") {
            if let Some(n) = t.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()) {
                installed += n;
            }
        } else if t.starts_with("Resolved ") && t.contains("package") {
            if let Some(n) = t.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()) {
                resolved += n;
            }
        } else if t.starts_with("error") || t.starts_with("warning") || t.starts_with("  x ") {
            warnings.push(line.to_string());
        }
        // Drop: progress bars, "Downloading", "Building", "Audited"
    }

    let mut out: Vec<String> = warnings;
    if installed > 0 {
        out.push(format!("[uv install complete — {} packages installed, {} resolved]", installed, resolved));
    } else if resolved > 0 {
        out.push(format!("[uv: {} packages already satisfied]", resolved));
    } else {
        return output.to_string();
    }
    out.join("\n")
}
