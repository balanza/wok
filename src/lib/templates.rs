pub fn compile_template(
    template: &str,
    params: &std::collections::HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut result = template.to_string();
    for (key, value) in params {
        // Tags are in the format __KEY__
        result = result.replace(&format!("__{}__", key), value);
    }
    Ok(result)
}
