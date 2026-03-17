use super::util;
use super::Handler;

pub struct HelmHandler;

impl Handler for HelmHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "list" | "ls" => filter_list(output),
            "install" | "upgrade" | "rollback" => filter_deploy(output),
            "uninstall" | "delete" => filter_uninstall(output),
            "status" => filter_status(output),
            "diff" => filter_diff(output),
            "template" => filter_template(output),
            _ => output.to_string(),
        }
    }
}

fn filter_list(output: &str) -> String {
    // Columns: NAME NAMESPACE REVISION UPDATED STATUS CHART APP VERSION
    // Keep: NAME, STATUS, CHART, NAMESPACE
    util::compact_table(output, &[0, 4, 5, 1])
}

fn filter_deploy(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("NAME:")
            || t.starts_with("LAST DEPLOYED:")
            || t.starts_with("NAMESPACE:")
            || t.starts_with("STATUS:")
            || t.starts_with("REVISION:")
            || t.starts_with("NOTES:")
            || t.starts_with("Release")
            || t.contains("has been")
            || t.contains("upgrade")
            || t.contains("deployed")
            || util::is_hard_keep(t)
        {
            out.push(line.to_string());
        }
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
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn filter_status(output: &str) -> String {
    let keep_keys = [
        "NAME:", "LAST DEPLOYED:", "NAMESPACE:", "STATUS:", "REVISION:",
        "NOTES:", "READY:", "AVAILABLE:", "AGE:",
    ];
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if keep_keys.iter().any(|k| t.starts_with(k)) || util::is_hard_keep(t) {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_diff(output: &str) -> String {
    // helm diff produces unified diff format — reuse diff logic
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("@@")
            || line.starts_with('+')
            || line.starts_with('-')
            || line.starts_with("Release")
            || line.starts_with("Chart")
        {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        "[no diff]".to_string()
    } else {
        out.join("\n")
    }
}

fn filter_template(output: &str) -> String {
    // helm template can produce huge YAML — show first 60 + summary
    let lines: Vec<&str> = output.lines().collect();
    let n = lines.len();
    if n <= 60 {
        return output.to_string();
    }
    let resource_count = output.matches("kind:").count();
    let mut out: Vec<String> = lines.iter().take(60).map(|l| l.to_string()).collect();
    out.push(format!(
        "[... {} lines total, {} Kubernetes resources]",
        n, resource_count
    ));
    out.join("\n")
}
