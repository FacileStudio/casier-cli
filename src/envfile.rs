use std::collections::BTreeMap;

pub fn parse(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        map.insert(key.to_string(), value.to_string());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_env_format() {
        let content = r#"
# comment
FOO=bar

export BAZ=qux
QUOTED="hello world"
SINGLE='single'
EMPTY=
NOEQUALS
  SPACED = padded
"#;
        let map = parse(content);
        assert_eq!(map.get("FOO").unwrap(), "bar");
        assert_eq!(map.get("BAZ").unwrap(), "qux");
        assert_eq!(map.get("QUOTED").unwrap(), "hello world");
        assert_eq!(map.get("SINGLE").unwrap(), "single");
        assert_eq!(map.get("EMPTY").unwrap(), "");
        assert_eq!(map.get("SPACED").unwrap(), "padded");
        assert!(!map.contains_key("NOEQUALS"));
        assert_eq!(map.len(), 6);
    }
}
