use super::util;
use super::Handler;

pub struct JournalctlHandler;

impl Handler for JournalctlHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let mut out = args.to_vec();
        // Prevent interactive paging
        if !args.iter().any(|a| a == "--no-pager") {
            out.push("--no-pager".to_string());
        }
        // Limit output if no -n / --lines already set
        if !args.iter().any(|a| a == "-n" || a.starts_with("--lines")) {
            out.push("-n".to_string());
            out.push("200".to_string());
        }
        out
    }

    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.is_empty() {
            return output.to_string();
        }

        let non_empty: Vec<(usize, &str)> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, l)| (i, *l))
            .collect();

        if non_empty.is_empty() {
            return output.to_string();
        }

        let texts: Vec<&str> = non_empty.iter().map(|(_, l)| *l).collect();

        match ccr_core::summarizer::embed_batch(&texts) {
            Ok(embeddings) => {
                let threshold = 0.90f32;
                let mut kept_indices: Vec<usize> = Vec::new();
                let mut kept_embeddings: Vec<Vec<f32>> = Vec::new();

                for (pos, (orig_idx, line)) in non_empty.iter().enumerate() {
                    if util::is_hard_keep(line) {
                        kept_indices.push(*orig_idx);
                        kept_embeddings.push(embeddings[pos].clone());
                        continue;
                    }
                    let is_dup = kept_embeddings
                        .iter()
                        .any(|e| util::cosine_similarity(&embeddings[pos], e) > threshold);
                    if !is_dup {
                        kept_indices.push(*orig_idx);
                        kept_embeddings.push(embeddings[pos].clone());
                    }
                }

                kept_indices.sort();
                let original_count = lines.len();
                let deduped: Vec<String> =
                    kept_indices.iter().map(|&i| lines[i].to_string()).collect();
                let mut result = deduped.join("\n");
                if deduped.len() < original_count {
                    result.push_str(&format!(
                        "\n[{} duplicate lines collapsed by semantic dedup]",
                        original_count - deduped.len()
                    ));
                }
                result
            }
            Err(_) => {
                // Fallback: keep errors + last 20 lines
                let error_lines: Vec<&str> = lines
                    .iter()
                    .filter(|&&l| util::is_hard_keep(l))
                    .copied()
                    .collect();
                let tail: Vec<&str> = lines.iter().rev().take(20).rev().copied().collect();

                let mut seen = std::collections::HashSet::new();
                let mut out: Vec<String> = Vec::new();
                for l in error_lines.iter().chain(tail.iter()) {
                    if seen.insert(*l) {
                        out.push(l.to_string());
                    }
                }
                out.join("\n")
            }
        }
    }
}
