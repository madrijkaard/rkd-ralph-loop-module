use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "VARCHAR", rename_all = "UPPERCASE")]
pub enum Status {
    A,
    I,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskType {
    JAVA,
    XML,
    SHELL_SCRIPT,
}

impl TaskType {

    pub fn values() -> Vec<&'static str> {
        vec![
            "JAVA",
            "XML",
            "SHELL_SCRIPT",
        ]
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_uppercase().as_str() {
            "JAVA" => Some(TaskType::JAVA),
            "XML" => Some(TaskType::XML),
            "SHELL_SCRIPT" => Some(TaskType::SHELL_SCRIPT),
            _ => None,
        }
    }

    pub fn is_valid(value: &str) -> bool {
        Self::from_str(value).is_some()
    }
}