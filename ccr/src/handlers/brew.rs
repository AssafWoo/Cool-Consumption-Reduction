use super::Handler;

pub struct BrewHandler;

impl Handler for BrewHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "install" | "reinstall" | "upgrade" => filter_install(output),
            "uninstall" | "remove" | "rm" => filter_uninstall(output),
            "update" => filter_update(output),
            "list" | "ls" => filter_list(output),
            "info" => filter_info(output),
            "doctor" | "config" => output.to_string(), // diagnostic — keep all
            _ => output.to_string(),
        }
    }
}

fn filter_install(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_caveats = false;

    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() && !in_caveats {
            continue;
        }

        // Caveats section — always keep completely
        if t == "==> Caveats" {
            in_caveats = true;
            out.push(line.to_string());
            continue;
        }
        if in_caveats {
            if t.starts_with("==>") && t != "==> Caveats" {
                in_caveats = false;
                // fall through to process this line
            } else {
                out.push(line.to_string());
                continue;
            }
        }

        // Key result lines
        if t.starts_with("==> Installing")
            || t.starts_with("==> Upgrading")
            || t.starts_with("==> Pouring")
            || t.starts_with("Already installed")
            || t.contains("is already installed")
            || t.starts_with("Warning:")
            || t.starts_with("Error:")
            || t.starts_with("Error ")
            || t.contains("successfully installed")
            || (t.starts_with("🍺") || t.contains("installed to"))
        {
            out.push(line.to_string());
        }
        // Drop: progress bars (==> Downloading, curl progress), SHA verification lines,
        //        ==> Checking..., checksum lines, "Fetching ..."
    }

    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_uninstall(output: &str) -> String {
    output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && (t.starts_with("Uninstalling") || t.starts_with("Error") || t.contains("uninstalled"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_update(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // Keep: "Updated N taps", "==> Updated Formulae", formula names, warnings
        if t.starts_with("Updated")
            || t.starts_with("Already up-to-date")
            || t.starts_with("==> Updated")
            || t.starts_with("==> New")
            || t.starts_with("==> Renamed")
            || t.starts_with("==> Deleted")
            || t.starts_with("Warning:")
            || t.starts_with("Error:")
        {
            out.push(line.to_string());
            continue;
        }
        // Formula name lines (indented or plain names under section headers)
        if line.starts_with(' ') || line.starts_with('\t') {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        "[brew update complete]".to_string()
    } else {
        out.join("\n")
    }
}

fn filter_list(output: &str) -> String {
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    let total = lines.len();
    const MAX: usize = 40;
    if total <= MAX {
        return output.to_string();
    }
    let mut out: Vec<String> = lines.iter().take(MAX).map(|l| l.to_string()).collect();
    out.push(format!("[+{} more packages]", total - MAX));
    out.join("\n")
}

fn filter_info(output: &str) -> String {
    // Keep: name/version line, description, homepage, installed status, dependencies
    // Drop: analytics, bottle info, long build options
    let mut out: Vec<String> = Vec::new();
    let mut line_count = 0usize;
    let mut past_analytics = false;

    for line in output.lines() {
        let t = line.trim();
        if t.starts_with("Analytics:") || t.starts_with("==> Analytics") {
            past_analytics = true;
            continue;
        }
        if past_analytics {
            continue;
        }
        if t.starts_with("==> Options") || t.starts_with("==> Caveats") {
            out.push(line.to_string());
            continue;
        }
        if line_count < 20 {
            out.push(line.to_string());
            line_count += 1;
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}
