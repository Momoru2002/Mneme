//! The user `Role` enum. Authentication/RBAC was removed; what remains is the
//! role type itself, still stored in `users.role` via `services::users::insert_user`
//! (dormant — no production reader currently).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl Role {
    /// The lowercase string stored in the `users.role` column / sent on the wire.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_as_str_is_lowercase() {
        assert_eq!(Role::Admin.as_str(), "admin");
        assert_eq!(Role::Editor.as_str(), "editor");
        assert_eq!(Role::Viewer.as_str(), "viewer");
    }
}
