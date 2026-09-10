use crate::{
    db::get_one,
    entities::user::{self as User, Role},
};
use actix_web::{HttpRequest, HttpResponse, Responder, get, web};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sha::sha256;
use openssl::sign::Signer;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use serde::Serialize;
use std::env;

use crate::api::user::PassRequest;

pub fn compute_hash(record: &crate::entities::audit::Model) -> String {
    let payload_str = record
        .payload
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_default();
    let raw = format!(
        "{}|{}|{:?}|{}|{:?}|{}|{}",
        record.id,
        record.occurred_at,
        record.actor_id,
        record.action,
        record.target_id,
        payload_str,
        record.previous_hash
    );
    let digest = sha256(raw.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn log_audit(
    db: &DatabaseConnection,
    mut new_log: crate::entities::audit::ActiveModel,
) -> Result<(), sea_orm::DbErr> {
    let last_record = crate::entities::audit::Entity::find()
        .order_by_desc(crate::entities::audit::Column::OccurredAt)
        .one(db)
        .await?;

    let prev_hash = match last_record {
        Some(record) => compute_hash(&record),
        None => "GENESIS".to_string(),
    };

    new_log.previous_hash = ActiveValue::Set(prev_hash);
    new_log.insert(db).await?;
    Ok(())
}

#[derive(Serialize)]
pub struct AuditExport {
    pub data: Vec<crate::entities::audit::Model>,
    pub signature: String,
}

#[get("/v1/admin/audit/export")]
pub async fn export_audit(
    req: HttpRequest,
    body: web::Json<PassRequest>,
    db: web::Data<DatabaseConnection>,
) -> impl Responder {
    let Some(auth) = req.headers().get("Authorization") else {
        return HttpResponse::Unauthorized().finish();
    };

    let Ok(auth) = auth.to_str() else {
        return HttpResponse::Unauthorized().finish();
    };

    let Some(token) = auth.strip_prefix("Bearer ") else {
        return HttpResponse::Unauthorized().finish();
    };

    let Ok(uuid) = uuid::Uuid::parse_str(token) else {
        return HttpResponse::BadRequest().finish();
    };

    let query = User::Entity::find().filter(User::Column::Id.eq(uuid));

    match get_one(db.get_ref(), query).await {
        Ok(Some(u)) => {
            if u.role == Role::Admin {
                let logs = match crate::entities::audit::Entity::find()
                    .order_by_asc(crate::entities::audit::Column::OccurredAt)
                    .all(db.get_ref())
                    .await
                {
                    Ok(l) => l,
                    Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
                };

                let json_data = serde_json::to_string(&logs).unwrap_or_default();

                let secret_key = env::var("SECRET").unwrap_or("default_unsafe_key".into());
                let pkey = PKey::hmac(secret_key.as_bytes()).unwrap();
                let mut signer = Signer::new(MessageDigest::sha256(), &pkey).unwrap();
                signer.update(json_data.as_bytes()).unwrap();

                let signature = signer
                    .sign_to_vec()
                    .unwrap()
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();

                HttpResponse::Ok().json(AuditExport {
                    data: logs,
                    signature,
                })
            } else {
                HttpResponse::BadRequest().finish()
            }
        }
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
