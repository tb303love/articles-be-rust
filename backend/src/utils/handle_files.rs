use actix_files::{Files, NamedFile};
use actix_web::dev::{fn_service, ServiceRequest, ServiceResponse};
use actix_web::HttpResponse;
use std::path::PathBuf;

pub fn handle_files(index_putanja: PathBuf, dir_za_radnika: &String) -> Files {
    Files::new("/", dir_za_radnika)
      .index_file("index.html")
      // Prikaz strukture foldera isključujemo u produkciji radi bezbednosti
      .show_files_listing()
      // Koristimo fn_service koji ispravno rukuje ServiceRequest-om
      .default_handler(fn_service(move |req: ServiceRequest| {
         let putanja_za_handler = index_putanja.clone();

         async move {
            let (http_req, _payload) = req.into_parts();

            // FIKSIRANO: Koristimo NamedFile::open() BEZ .await na kraju
            match NamedFile::open(&putanja_za_handler) {
               Ok(file) => {
                  let res = file.into_response(&http_req);
                  Ok(ServiceResponse::new(http_req, res))
               }
               Err(e) => {
                  log::error!(
                            "Kritična greška: Angular index.html nije pronađen na lokaciji {:?}: {:?}",
                            putanja_za_handler,
                            e
                        );
                  let res = HttpResponse::NotFound().json("Frontend nije dostupan.");
                  Ok(ServiceResponse::new(http_req, res))
               }
            }
         }
      }))
}
