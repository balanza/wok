use super::projects::ProjectItem;
use std::io::{self, Write};

type ListWriterFn = fn(Vec<ProjectItem>, &mut dyn Write) -> io::Result<()>;

pub fn get_writer_fn(format: &str) -> io::Result<ListWriterFn> {
    match format {
        "json" => Ok(write_project_tree_json),
        "flat" => Ok(write_project_tree_flat),
        "tree" => Ok(write_project_tree_compact),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported format: {}", format),
        )),
    }
}

fn write_project_tree_compact(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    if list.is_empty() {
        return Ok(());
    }

    // sorting by id ensures that we maintain the order of projects within each organization
    // This is important for the tree structure to be consistent.
    let sorted_list = to_sorted_project_list(list);

    let mut current_org: Option<&str> = None;

    for (i, item) in sorted_list.iter().enumerate() {
        // Check if we're starting a new organization
        if current_org != Some(&item.organization) {
            // If we had a previous org, fix up the last project's prefix
            if i > 0 {
                // The last project of previous org needs └── instead of ├──
                // We'll handle this by looking ahead
            }

            // Write the organization name
            writeln!(writer, "{}", item.organization)?;
            current_org = Some(&item.organization);
        }

        // Check if this is the last project in the current org
        let is_last_in_org =
            i == sorted_list.len() - 1 || sorted_list[i + 1].organization != item.organization;

        let prefix = if is_last_in_org {
            "└──"
        } else {
            "├──"
        };

        let marker = if item.is_current { "* " } else { "" };

        writeln!(
            writer,
            "{} {}{} ({})",
            prefix,
            marker,
            item.name,
            item.remote.as_deref().unwrap_or("no remote")
        )?;
    }

    Ok(())
}

fn write_project_tree_json(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    let result = serde_json::to_writer(writer, &list.clone());

    if let Err(e) = result {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to write JSON: {}", e),
        ));
    }
    Ok(())
}

fn write_project_tree_flat(list: Vec<ProjectItem>, writer: &mut dyn Write) -> io::Result<()> {
    for item in list {
        let marker = if item.is_current { "* " } else { "" };
        writeln!(
            writer,
            "{}{} ({})",
            marker,
            item.name,
            item.remote.as_deref().unwrap_or("no remote")
        )?;
    }
    Ok(())
}

fn to_sorted_project_list(list: Vec<ProjectItem>) -> Vec<ProjectItem> {
    let mut sorted_list = list.clone();
    sorted_list.sort_by(|a, b| a.id.cmp(&b.id));
    sorted_list
}

mod tests {
    #[allow(unused_imports)]
    use super::{get_writer_fn, ProjectItem};

    #[test]
    fn test_get_writer_function() {
        assert!(get_writer_fn("json").is_ok());
        assert!(get_writer_fn("flat").is_ok());
        assert!(get_writer_fn("tree").is_ok());
        assert!(get_writer_fn("invalid").is_err());
    }

    // tree

    #[test]
    fn test_write_empty_list_tree() {
        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        let result = writer_fn(vec![], &mut output);

        assert!(result.is_ok());
        assert!(output.is_empty());
    }

    #[test]
    fn test_write_single_org_tree() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project2".to_string(),
                name: "project2".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project2.git".to_string()),
                is_current: true,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines[0], "org1");
        assert_eq!(
            lines[1],
            "├── project1 (https://github.com/org1/project1.git)"
        );
        assert_eq!(
            lines[2],
            "└── * project2 (https://github.com/org1/project2.git)"
        );
    }

    #[test]
    fn test_write_multiple_orgs_tree() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org2::project2".to_string(),
                name: "project2".to_string(),
                organization: "org2".to_string(),
                remote: Some("https://github.com/org2/project2.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project3".to_string(),
                name: "project3".to_string(),
                organization: "org1".to_string(),
                remote: None,
                is_current: false,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        // Should be sorted by ID, so org1 projects come first
        assert_eq!(lines[0], "org1");
        assert_eq!(
            lines[1],
            "├── project1 (https://github.com/org1/project1.git)"
        );
        assert_eq!(lines[2], "└── project3 (no remote)");
        assert_eq!(lines[3], "org2");
        assert_eq!(
            lines[4],
            "└── project2 (https://github.com/org2/project2.git)"
        );
    }

    #[test]
    fn test_write_no_remote_tree() {
        let items = vec![ProjectItem {
            id: "org1::project1".to_string(),
            name: "project1".to_string(),
            organization: "org1".to_string(),
            remote: None,
            is_current: false,
        }];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("tree").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("(no remote)"));
    }

    // json
    #[test]
    fn test_write_empty_list_json() {
        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        let result = writer_fn(vec![], &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.eq("[]"));
    }

    #[test]
    fn test_write_single_org_json() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project2".to_string(),
                name: "project2".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project2.git".to_string()),
                is_current: true,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":"https://github.com/org1/project1.git","is_current":false},{"id":"org1::project2","name":"project2","organization":"org1","remote":"https://github.com/org1/project2.git","is_current":true}]"#;
        assert_eq!(output_str, expected);
    }

    #[test]
    fn test_write_multiple_orgs_json() {
        let items = vec![
            ProjectItem {
                id: "org1::project1".to_string(),
                name: "project1".to_string(),
                organization: "org1".to_string(),
                remote: Some("https://github.com/org1/project1.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org2::project2".to_string(),
                name: "project2".to_string(),
                organization: "org2".to_string(),
                remote: Some("https://github.com/org2/project2.git".to_string()),
                is_current: false,
            },
            ProjectItem {
                id: "org1::project3".to_string(),
                name: "project3".to_string(),
                organization: "org1".to_string(),
                remote: None,
                is_current: false,
            },
        ];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":"https://github.com/org1/project1.git","is_current":false},{"id":"org2::project2","name":"project2","organization":"org2","remote":"https://github.com/org2/project2.git","is_current":false},{"id":"org1::project3","name":"project3","organization":"org1","remote":null,"is_current":false}]"#;
        assert_eq!(output_str, expected);
    }

    #[test]
    fn test_write_no_remote_json() {
        let items = vec![ProjectItem {
            id: "org1::project1".to_string(),
            name: "project1".to_string(),
            organization: "org1".to_string(),
            remote: None,
            is_current: false,
        }];

        let mut output = Vec::new();
        let writer_fn = get_writer_fn("json").unwrap();
        writer_fn(items, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        let expected = r#"[{"id":"org1::project1","name":"project1","organization":"org1","remote":null,"is_current":false}]"#;
        assert_eq!(output_str, expected);
    }

    // flat
    // TODO: Implement tests for flat format similar to tree and json
    // not important for now, but should be added later
}
