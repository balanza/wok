pub fn handle(
    _workspace: &str,
    orgs: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(orgs) = orgs {
        println!("Fast-forwarding main branches for orgs: {:?}", orgs);
    } else {
        println!("Fast-forwarding all main branches");
    }
    Err("Not implemented yet".into())
}
