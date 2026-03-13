use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::BackupCodesResponse;
use crate::dto::auth::RegenerateBackupCodesRequest;

#[post("/backup-codes/regenerate", data = "<_payload>")]
pub async fn regenerate_backup_codes(_user: CurrentUser, _payload: Json<RegenerateBackupCodesRequest>) -> Json<BackupCodesResponse> {
    todo!()
}
