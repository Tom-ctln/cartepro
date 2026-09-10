use crate::{
    db::get_all,
    entities::transaction::{self as Transaction},
};
use actix_web::{HttpResponse, Responder, get, web};
use csv::Writer;
use sea_orm::DatabaseConnection;

#[utoipa::path(
    get,
    path = "/api/v1/admin/transactions.csv",
    responses(
        (status = 200, description = "CSV file containing all transactions", content_type = "text/csv"),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/v1/admin/transactions.csv")]
pub async fn transactions_to_csv(db: web::Data<DatabaseConnection>) -> impl Responder {
    let transactions = match get_all::<Transaction::Entity, _>(db.get_ref()).await {
        Ok(transactions) => transactions,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    let mut writer = Writer::from_writer(Vec::new());

    for transaction in transactions {
        writer.serialize(transaction).ok();
    }

    match writer.into_inner() {
        Ok(file) => HttpResponse::Ok().content_type("text/csv").body(file),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
