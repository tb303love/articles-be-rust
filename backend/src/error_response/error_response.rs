use actix_web::{
    body::MessageBody, dev::ServiceResponse, middleware::ErrorHandlerResponse, HttpResponse,
};

pub fn _404<B: MessageBody + 'static>(
    res: ServiceResponse<B>,
) -> actix_web::Result<ErrorHandlerResponse<B>> {
    let (req, res_head) = res.into_parts();
    let novi_http_res = HttpResponse::NotFound().body("Putanja ne postoji.");

    Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
        req,
        res_head
            .set_body(novi_http_res.into_body())
            .map_into_right_body(),
    )))
}
