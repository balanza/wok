use wok::lib::constants::GOTO_MARKER;
use wok::lib::fuzzy::FuzzySearcher;
use wok::lib::list_writer::get_writer_fn;
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
        let write_fn = get_writer_fn("flat")?;
        write_fn(filtered_list, &mut std::io::stdout())?;
        return Err(Box::from("Multiple projects found. Please refine your search or use a different command to select a project."));
    }

    println!("{}", to_goto(workspace, filtered_list[0].clone()));

    Ok(())
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

fn to_goto(workspace: &str, item: ProjectItem) -> String {
    format!(
        "{}{}/{}/{}",
        GOTO_MARKER, workspace, item.organization, item.name
    )
}

mod tests {
    #[allow(unused_imports)]
    use super::{fuzzy_search, to_goto, ProjectItem};

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
