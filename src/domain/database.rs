use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Deserialize, Debug)]
pub struct NewUser {
    pub name: String,
}

// Two users created together: both are saved, or neither is.
#[derive(Deserialize, Debug)]
pub struct NewUserPair {
    pub first: String,
    pub second: String,
}

#[derive(Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub uid: i32,
    pub name: String,
}
