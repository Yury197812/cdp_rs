// database/sqlite/query.rs - Query results

use std::collections::HashMap;

pub struct QueryResult {
    pub rows: Vec<HashMap<String, String>>,
}

impl QueryResult {
    pub fn new() -> Self {
        QueryResult { rows: Vec::new() }
    }
    
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    
    pub fn get(&self, index: usize) -> Option<&HashMap<String, String>> {
        self.rows.get(index)
    }
    
    pub fn push(&mut self, row: HashMap<String, String>) {
        self.rows.push(row);
    }
    
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.rows).unwrap_or_default()
    }
}

impl std::fmt::Display for QueryResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QueryResult({} rows)", self.rows.len())
    }
}
