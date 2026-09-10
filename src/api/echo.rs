use actix_web::{HttpResponse, Responder, post};

#[utoipa::path(
    post,
    path = "/api/echo",
    request_body = String,
    responses(
        (status = 200, description = "Echo response", content_type = "text/plain")
    )
)]
#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}
