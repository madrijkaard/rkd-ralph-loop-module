use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "VARCHAR", rename_all = "UPPERCASE")]
pub enum Status {
    A,
    I,
}