use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::OperatorId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRole {
    Viewer,
    Dispatcher,
    Operator,
    Administrator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ReadOperations,
    ManageAlerts,
    ControlReplay,
    ReadMetrics,
    ManageMemberships,
}

impl AuthRole {
    pub const fn permits(self, permission: Permission) -> bool {
        match permission {
            Permission::ReadOperations => true,
            Permission::ManageAlerts => matches!(
                self,
                Self::Dispatcher | Self::Operator | Self::Administrator
            ),
            Permission::ControlReplay | Permission::ReadMetrics => {
                matches!(self, Self::Operator | Self::Administrator)
            }
            Permission::ManageMemberships => matches!(self, Self::Administrator),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Dispatcher => "dispatcher",
            Self::Operator => "operator",
            Self::Administrator => "administrator",
        }
    }
}

impl TryFrom<&str> for AuthRole {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "viewer" => Ok(Self::Viewer),
            "dispatcher" => Ok(Self::Dispatcher),
            "operator" => Ok(Self::Operator),
            "administrator" => Ok(Self::Administrator),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthContext {
    pub identity_id: Uuid,
    pub operator_id: OperatorId,
    pub operator_code: String,
    pub operator_name: String,
    pub provider: String,
    pub subject: String,
    pub session_id: String,
    pub role: AuthRole,
}

impl AuthContext {
    pub const fn permits(&self, permission: Permission) -> bool {
        self.role.permits(permission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_matrix_is_explicit_and_least_privilege() {
        assert!(AuthRole::Viewer.permits(Permission::ReadOperations));
        assert!(!AuthRole::Viewer.permits(Permission::ManageAlerts));
        assert!(AuthRole::Dispatcher.permits(Permission::ManageAlerts));
        assert!(!AuthRole::Dispatcher.permits(Permission::ControlReplay));
        assert!(AuthRole::Operator.permits(Permission::ControlReplay));
        assert!(!AuthRole::Operator.permits(Permission::ManageMemberships));
        assert!(AuthRole::Administrator.permits(Permission::ManageMemberships));
    }
}
