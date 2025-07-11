use serde::{Deserialize, Serialize};

use crate::primitives::{PermissionSetId, RoleId, UserId};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CoreAccessEvent {
    UserCreated {
        id: UserId,
        email: String,
    },
    UserRemoved {
        id: UserId,
    },
    UserGrantedRole {
        id: UserId,
        role_id: RoleId,
    },
    UserRevokedRole {
        id: UserId,
        role_id: RoleId,
    },

    RoleCreated {
        id: RoleId,
        name: String,
    },
    RoleGainedPermissionSet {
        id: RoleId,
        permission_set_id: PermissionSetId,
    },
    RoleLostPermissionSet {
        id: RoleId,
        permission_set_id: PermissionSetId,
    },
}
