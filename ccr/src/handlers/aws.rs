use super::util;
use super::Handler;

pub struct AwsHandler;

// Subcommands that return structured JSON output and accept --output json.
// S3 transfer commands, configure, and help do not.
const JSON_SUBCMDS: &[&str] = &[
    "ec2", "ecs", "eks", "lambda", "s3api", "iam", "rds", "elb", "elbv2",
    "cloudformation", "cloudwatch", "sns", "sqs", "sts", "ssm", "secretsmanager",
    "route53", "logs", "dynamodb", "kinesis", "glue", "emr", "athena",
];

impl Handler for AwsHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        let should_inject = JSON_SUBCMDS.contains(&subcmd)
            && !args.iter().any(|a| a == "--output");
        if should_inject {
            let mut out = args.to_vec();
            out.push("--output".to_string());
            out.push("json".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");

        // Always preserve errors
        if output.trim_start().starts_with("An error")
            || (output.contains("Error") && output.contains("Code"))
        {
            return output.to_string();
        }

        if subcmd == "s3" && action == "ls" {
            return filter_s3_ls(output);
        }

        // JSON output: apply schema extraction
        let trimmed = output.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let schema = util::json_to_schema(&v);
                let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_default();
                if schema_str.len() < trimmed.len() {
                    return schema_str;
                }
            }
        }

        output.to_string()
    }
}

fn filter_s3_ls(output: &str) -> String {
    let mut prefixes: std::collections::HashMap<String, (usize, u64)> =
        std::collections::HashMap::new();
    let mut loose_count = 0usize;
    let mut loose_size = 0u64;

    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("PRE ") {
            let prefix = t[4..].trim().to_string();
            prefixes.entry(prefix).or_insert((0, 0));
            continue;
        }
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(size) = parts[2].parse::<u64>() {
                loose_count += 1;
                loose_size += size;
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    if loose_count > 0 {
        out.push(format!("{} objects, {} bytes", loose_count, loose_size));
    }
    for (prefix, (count, size)) in &prefixes {
        if *count > 0 {
            out.push(format!("{}: {} objects, {} bytes", prefix, count, size));
        } else {
            out.push(format!("{}/", prefix));
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.sort();
        out.join("\n")
    }
}
