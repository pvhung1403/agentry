use std::collections::HashMap;

/// Minimal YAML frontmatter reader: top-level `key: value` pairs and indented block values
/// (`|`, `>`), joined into one line. Enough for `name` / `description` in SKILL.md and agent files.
pub fn parse(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next().map(str::trim) != Some("---") {
        return out;
    }

    let mut block_key: Option<String> = None;
    let mut block: Vec<String> = Vec::new();
    fn flush(key: &mut Option<String>, block: &mut Vec<String>, out: &mut HashMap<String, String>) {
        if let Some(key) = key.take() {
            out.insert(key, block.join(" "));
        }
        block.clear();
    }

    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if block_key.is_some() && indented {
            block.push(line.trim().to_string());
            continue;
        }
        flush(&mut block_key, &mut block, &mut out);
        if indented || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.trim().to_string();
        let mut value = value.trim();
        if ["", "|", ">", "|-", ">-", "|+", ">+"].contains(&value) {
            block_key = Some(key);
            continue;
        }
        let quoted = value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')));
        if quoted {
            value = &value[1..value.len() - 1];
        }
        out.insert(key, value.to_string());
    }
    flush(&mut block_key, &mut block, &mut out);
    out
}
