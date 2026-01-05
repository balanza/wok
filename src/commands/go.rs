use std::os::unix::io::AsRawFd;
use wok::lib::constants::{GOTO_MARKER, MULTIPLE_MATCHES_MARKER};
use wok::lib::fd::write_to_fd;
use wok::lib::fuzzy::FuzzySearcher;
use wok::lib::projects::{list_project_items, ProjectItem};

pub fn handle(workspace: &str, search: &str) -> Result<(), Box<dyn std::error::Error>> {
    let list: Vec<ProjectItem> = list_project_items(workspace, vec![])?;
    let filtered_list = if search.is_empty() {
        list
    } else {
        fuzzy_search(&list, search)
    };

    if filtered_list.is_empty() {
        return Err(Box::from("No projects found matching the search criteria."));
    }

    if filtered_list.len() > 1 {
        // Write marker to fd 3 to signal multiple matches to the shell wrapper
        // Format: MARKER followed by newline-separated list of matches
        let matches: Vec<String> = filtered_list
            .iter()
            .map(|item| format!("{}/{}", item.organization, item.name))
            .collect();
        let multiple_matches_data = format!("{}\n{}", MULTIPLE_MATCHES_MARKER, matches.join("\n"));
        write_to_fd(3, &multiple_matches_data)?;

        // Let the shell wrapper handle displaying the menu
        return Ok(());
    }

    let dest = format!(
        "{}/{}/{}",
        workspace, filtered_list[0].organization, filtered_list[0].name
    );

    let goto_dest = format!("{}{}", GOTO_MARKER, &dest);

    // Write to fd 3 for the shell to pick up
    write_to_fd(3, &goto_dest)?;

    // Also print to stdout for use the output in other contexts
    if is_tty() {
        println!("{}", &dest);
    }

    Ok(())
}

fn is_tty() -> bool {
    unsafe { libc::isatty(std::io::stdout().as_raw_fd()) != 0 }
}

fn fuzzy_search(list: &Vec<ProjectItem>, search: &str) -> Vec<ProjectItem> {
    let flat_list = list
        .iter()
        .map(|item| format!("{}/{}", item.organization, item.name))
        .collect::<Vec<_>>();
    let searcher = FuzzySearcher::new(0.6);
    searcher
        .search(search, &flat_list)
        .into_iter()
        .map(|m| m.text)
        .filter_map(|text| {
            list.iter()
                .find(|item| format!("{}/{}", item.organization, item.name) == text)
                .cloned()
        })
        .collect()
}

mod tests {
    #[allow(unused_imports)]
    use super::{fuzzy_search, ProjectItem};

    // somehow this function is marked as dead code, but it is used in the tests
    #[allow(dead_code)]
    fn get_list() -> Vec<ProjectItem> {
        return vec![
            ProjectItem {
                id: "zoo::project1".to_string(),
                name: "project1".to_string(),
                organization: "zoo".to_string(),
                remote: None,
                is_current: false,
            },
            ProjectItem {
                id: "aardvark::project2".to_string(),
                name: "project2".to_string(),
                organization: "aardvark".to_string(),
                remote: None,
                is_current: false,
            },
            ProjectItem {
                id: "middle::project3".to_string(),
                name: "project3".to_string(),
                organization: "middle".to_string(),
                remote: None,
                is_current: false,
            },
        ];
    }

    #[test]
    fn test_fuzzy_search_exact_match() {
        let results = fuzzy_search(&get_list(), "project1");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "project1");
    }

    #[test]
    fn test_fuzzy_search_partial_match() {
        let results = fuzzy_search(&get_list(), "proj");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "project1");
        assert_eq!(results[1].name, "project2");
        assert_eq!(results[2].name, "project3");
    }

    #[test]
    fn test_fuzzy_search_fuzzy_match() {
        let results = fuzzy_search(&get_list(), "arkprj");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "project2");
    }

    #[test]
    fn test_fuzzy_search_no_match() {
        let results = fuzzy_search(&get_list(), "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_search_empty_search() {
        let results = fuzzy_search(&get_list(), "");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "project1");
        assert_eq!(results[1].name, "project2");
        assert_eq!(results[2].name, "project3");
    }

    #[test]
    fn test_fuzzy_search_similar_match() {
        let results = fuzzy_search(&get_list(), "akrprj");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "project2");
    }
}
