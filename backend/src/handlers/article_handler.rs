use crate::context::AppContext;
use crate::dtos::ArticleCreateRequest;
use actix_web::{get, post, put, web, HttpResponse, Responder};
use serde::Deserialize;

// --- STRUKTURE ZA AUTOMATSKU VALIDACIJU QUERY PARAMETARA ---

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
}

#[derive(Deserialize)]
pub struct NameCheckQuery {
    pub name: String,
}

#[derive(Deserialize)]
pub struct NameExcludeQuery {
    pub name: String,
    pub id: i64,
}

// --- HANDLERI ---

/// GET /api/articles?query=neki_tekst
#[get("/articles")]
pub async fn get_all_articles(
    ctx: web::Data<AppContext>,
    query: web::Query<SearchQuery>, // Automatski mapira opcioni tekst
) -> HttpResponse {
    match ctx
        .article_service
        .get_all_articles(query.into_inner().query)
        .await
    {
        Ok(articles) => HttpResponse::Ok().json(articles),
        Err(e) => {
            log::error!("Greška pri čitanju artikala iz baze: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

/// GET /api/articles/check-name?name=mleko
#[get("/articles/check-name")]
pub async fn check_name(
    ctx: web::Data<AppContext>,
    query: web::Query<NameCheckQuery>, // Garantuje da "name" parametar postoji
) -> impl Responder {
    match ctx
        .article_service
        .check_name_availability(&query.name)
        .await
    {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri imena artikla: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

/// GET /api/articles/check-name-exclude?name=mleko&id=5
#[get("/articles/check-name-exclude")]
pub async fn check_name_exclude(
    ctx: web::Data<AppContext>,
    query: web::Query<NameExcludeQuery>, // Sigurno parsira i ime i ID u i64
) -> impl Responder {
    let q = query.into_inner();
    match ctx
        .article_service
        .find_article_by_article_name_exclude_specific(&q.name, q.id)
        .await
    {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri izuzetog imena artikla: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

/// GET /api/articles/check-name-exclude-specific?name=mleko&id=5
#[get("/articles/check-name-exclude-specific")]
pub async fn check_name_exclude_exclude_specific(
    ctx: web::Data<AppContext>,
    query: web::Query<NameExcludeQuery>,
) -> impl Responder {
    let q = query.into_inner();
    match ctx
        .article_service
        .find_article_by_article_name_exclude_specific(&q.name, q.id)
        .await
    {
        Ok(is_available) => HttpResponse::Ok().json(is_available),
        Err(e) => {
            log::error!("Greška pri proveri specifičnog izuzetog imena: {:?}", e);
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

/// GET /api/articles/affected-bundles/{stock_id}
#[get("/articles/affected-bundles/{stock_id}")]
pub async fn find_affected_bundles(
    ctx: web::Data<AppContext>,
    path: web::Path<i64>,
) -> impl Responder {
    let stock_id = path.into_inner();

    match ctx.article_service.get_affected_bundles(stock_id).await {
        Ok(bundle_names) => HttpResponse::Ok().json(bundle_names),
        Err(e) => {
            log::error!(
                "Greška pri pronalaženju pogođenih paketa za ID {}: {:?}",
                stock_id,
                e
            );
            HttpResponse::InternalServerError().body("Greška na serveru")
        }
    }
}

/// POST /api/articles
#[post("/articles")]
pub async fn create_article(
    ctx: web::Data<AppContext>,
    body: web::Json<ArticleCreateRequest>,
) -> impl Responder {
    match ctx
        .article_service
        .check_name_availability(&body.name)
        .await
    {
        Ok(is_available) => {
            if !is_available {
                return HttpResponse::BadRequest().body("Artikal sa tim imenom već postoji");
            }
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    }

    match ctx.article_service.create_article(body.into_inner()).await {
        Ok(new_id) => HttpResponse::Created().json(new_id),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

/// PUT /api/articles/{id}
#[put("/articles/{id}")]
pub async fn update_article(
    ctx: web::Data<AppContext>,
    path: web::Path<i64>,
    body: web::Json<ArticleCreateRequest>,
) -> impl Responder {
    let article_id = path.into_inner();

    match ctx
        .article_service
        .update_article(article_id, body.into_inner())
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Artikal uspešno ažuriran"),
        Err(err) => match err {
            sqlx::Error::RowNotFound => HttpResponse::NotFound().body("Artikal nije pronađen"),
            _ => {
                log::error!("Greška pri ažuriranju artikla {}: {:?}", article_id, err);
                HttpResponse::InternalServerError().body(err.to_string())
            }
        },
    }
}

/// GET /api/articles/barcode/{code}
#[get("/articles/barcode/{code}")]
pub async fn find_by_barcode(
    ctx: web::Data<AppContext>,
    code: web::Path<String>,
) -> impl Responder {
    let scanned_barcode = code.into_inner();

    match ctx.article_service.find_by_barcode(&scanned_barcode).await {
        Ok(article) => HttpResponse::Ok().json(article),
        Err(err) => match err {
            sqlx::Error::RowNotFound => {
                HttpResponse::NotFound().body("Artikal sa tim bar-kodom nije pronađen")
            }
            _ => {
                log::error!(
                    "Greška u bazi pri traženju bar-koda {}: {:?}",
                    scanned_barcode,
                    err
                );
                HttpResponse::InternalServerError().body("Greška na serveru")
            }
        },
    }
}

/// DELETE /api/articles/{id}
#[actix_web::delete("/articles/{id}")]
pub async fn delete_article(ctx: web::Data<AppContext>, path: web::Path<i64>) -> impl Responder {
    let article_id = path.into_inner();

    match ctx.article_service.delete_by_id(article_id).await {
        Ok(_) => HttpResponse::Ok().body("Artikal uspešno obrisan"),
        Err(e) => {
            log::error!(
                "Greška pri brisanju artikla sa ID-jem {}: {:?}",
                article_id,
                e
            );
            HttpResponse::InternalServerError().body("Greška na serveru prilikom brisanja artikla")
        }
    }
}

/// Funkcija za grupisanje ruta pod /api prefiksom
// src/handlers/article_handler.rs

pub fn init_article_routes(cfg: &mut web::ServiceConfig) {
    // Ovde registrujemo samo servise za artikle.
    // Primeti da ovde NE stavljamo web::scope("/api") niti middleware.
    cfg.service(get_all_articles)
        .service(create_article)
        .service(update_article)
        .service(delete_article)
        .service(find_by_barcode)
        .service(check_name)
        .service(check_name_exclude)
        .service(check_name_exclude_exclude_specific)
        .service(find_affected_bundles);
}
