use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StompConfig {
    pub name: String,
    pub labels: HashMap<String, String>,
}
