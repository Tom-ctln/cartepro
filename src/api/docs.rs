use crate::api::ApiDoc;
use actix_web::{HttpResponse, Responder, get, web};
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config, SwaggerUi};

#[get("/openapi.json")]
async fn openapi() -> impl Responder {
    match serde_json::to_string(&ApiDoc::openapi()) {
        Ok(json) => HttpResponse::Ok()
            .content_type("application/json")
            .body(json),

        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to generate OpenAPI spec: {err}")),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/docs").service(openapi).service(
        SwaggerUi::new("/swagger-ui/{_:.*}").config(Config::new(["/api/docs/openapi.json"])),
    ));
}
