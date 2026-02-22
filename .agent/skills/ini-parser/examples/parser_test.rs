#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug, PartialEq)]
    enum KeyType { Cycle, Toggle, Hold, Unknown }

    #[derive(Debug)]
    struct KeyBind {
        section: String,
        group_name: String,
        interaction: KeyType,
        key: String,
        back_key: Option<String>,
        variables: HashMap<String, Vec<String>>,
        condition: Option<String>,
    }

 
    #[test]
    fn test_real_world_parsing() {
        let input = r#"
[KeySwap01]
condition = $active == 1
key = z
type = cycle
$Cape = 0,1,2,3,4,5,6,7,8,9,10
$creditinfo = 0

[KeySwap02]
condition = $active == 1
Key = 0
type = cycle
$Dress = 0,1,2,3,4

[KeySwap07]
condition = $active == 1
Key = ]
type = cycle
$Boots = 0,1
"#;
        let mut bindings = Vec::new();
        let mut current = KeyBind {
            section: String::new(),
            group_name: String::new(),
            interaction: KeyType::Unknown,
            key: String::new(),
            back_key: None,
            variables: HashMap::new(),
            condition: None,
        };

        for line in input.lines() {
            let trim = line.trim();
            if trim.is_empty() || trim.starts_with(';') { continue; }

            if trim.starts_with('[') && trim.ends_with(']') {
                if !current.section.is_empty() {
                    bindings.push(current);
                }
                
                let section = trim[1..trim.len()-1].to_string();
                // Normalize "KeySwap01" -> "Swap01"
                let group = section.strip_prefix("Key").unwrap_or(&section).to_string();
                
                current = KeyBind {
                    section,
                    group_name: group,
                    interaction: KeyType::Unknown,
                    key: String::new(),
                    back_key: None,
                    variables: HashMap::new(),
                    condition: None,
                };
            } else if !current.section.is_empty() {
                if let Some((k, v)) = trim.split_once('=') {
                    let key = k.trim().to_lowercase();
                    let val = v.trim().to_string();

                    match key.as_str() {
                        "key" => current.key = val,
                        "back" => current.back_key = Some(val),
                        "type" => current.interaction = match val.to_lowercase().as_str() {
                            "cycle" => KeyType::Cycle,
                            "toggle" => KeyType::Toggle,
                            "hold" => KeyType::Hold,
                            _ => KeyType::Unknown,
                        },
                        "condition" => current.condition = Some(val),
                        _ if key.starts_with('$') => {
                            let options: Vec<String> = val.split(',')
                                .map(|s| s.trim().to_string())
                                .collect();
                            current.variables.insert(key, options);
                        }
                        _ => {}
                    }
                }
            }
        }
        if !current.section.is_empty() { bindings.push(current); }

        assert_eq!(bindings.len(), 3);

        // Verify Cape Cycle (Long list)
        let cape = &bindings[0];
        assert_eq!(cape.group_name, "Swap01");
        assert_eq!(cape.key, "z");
        assert!(cape.variables.contains_key("$Cape"));
        assert_eq!(cape.variables["$Cape"].len(), 11); // 0..10
        assert_eq!(cape.variables["$Cape"][10], "10");

        // Verify Dress Cycle
        let dress = &bindings[1];
        assert_eq!(dress.key, "0");
        assert!(dress.variables.contains_key("$Dress"));
        assert_eq!(dress.variables["$Dress"].len(), 5);
    }
}
