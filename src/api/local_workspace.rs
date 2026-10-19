//! Personal settings and offline imports, with no account or remote setup.
use crate::{
    db::{DbPool, queries},
    error::LificError,
};
use axum::{Extension, Json, extract::State, http::HeaderMap};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) async fn get(State(db): State<DbPool>) -> Result<Json<Value>, LificError> {
    let name = {
        let conn = db.read()?;
        queries::settings::get(&conn)?
            .instance_name
            .unwrap_or_default()
    };
    let executable = std::env::current_exe().map_err(|e| LificError::Internal(e.to_string()))?;
    let database =
        std::path::absolute(db.path()).map_err(|e| LificError::Internal(e.to_string()))?;
    Ok(Json(
        json!({"name":name, "executable":executable, "database":database}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkspacePatch {
    name: String,
}

pub(super) async fn rename(
    State(db): State<DbPool>,
    headers: HeaderMap,
    Json(input): Json<WorkspacePatch>,
) -> Result<Json<Value>, LificError> {
    let name = input.name.trim();
    if name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err(LificError::BadRequest(
            "Workspace name must be at most 60 characters without control characters".into(),
        ));
    }
    let token = crate::auth::recent_session_token(&headers)?;
    db.transaction(|tx| {
        let operator = queries::users::first_admin(tx)?
            .ok_or_else(|| LificError::Forbidden("Local operator required".into()))?;
        crate::auth::revalidate_recent_session(tx, &token, operator.id)?;
        queries::settings::update(
            tx,
            queries::settings::InstanceSettingsPatch {
                instance_name: Some(name.into()),
                ..Default::default()
            },
        )?;
        Ok(())
    })?;
    get(State(db)).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportFile {
    source: crate::import::file::Source,
    project: String,
    namespace: Option<String>,
    data: Value,
    #[serde(default)]
    dry_run: bool,
}

pub(super) async fn import_file(
    State(db): State<DbPool>,
    Extension(identity): Extension<Option<crate::resolve_caller::ResolvedIdentity>>,
    Json(input): Json<ImportFile>,
) -> Result<Json<crate::import::ImportSummary>, LificError> {
    let project_id = {
        let conn = db.read()?;
        queries::resolve_project_identifier(&conn, &input.project)?
    };
    crate::authz::require_role(&db, &identity, project_id, crate::db::models::Role::Lead)?;
    let bytes =
        serde_json::to_vec(&input.data).map_err(|e| LificError::BadRequest(e.to_string()))?;
    let summary = crate::import::file::run(
        &db,
        &input.project,
        input.source,
        input.namespace.as_deref(),
        &bytes,
        input.dry_run,
    )?;
    Ok(Json(summary))
}
