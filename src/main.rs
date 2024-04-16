#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

use rocket::Data;
use rocket_contrib::json::Json;
use rocket_contrib::uuid::Uuid;
use rocket_contrib::serve::StaticFiles;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use local_ip_address::local_ip;
use qrrs::qrcode::{make_code, print_code_to_term, QrCodeViewArguments};
use rocket::data::ToByteUnit;
use rocket::http::ContentType;
use rocket::response::content::RawHtml;
use rocket::response::Redirect;
use rocket_multipart_form_data::{multer, MultipartFormData, MultipartFormDataError, MultipartFormDataField, MultipartFormDataOptions};
use rocket_multipart_form_data::mime::STAR;
use rocket_raw_response::mime::{IMAGE_STAR, STAR_STAR};
use rocket_raw_response::RawResponse;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(include_str!("index.html"))
}

#[post("/upload", data = "<data>")]
async fn upload(content_type: &ContentType, data: Data<'_>) -> Result<Redirect, &'static str> {
    let options = MultipartFormDataOptions {
        max_data_bytes: 33 * 1024 * 1024,
        allowed_fields: vec![MultipartFormDataField::raw("file")
            .size_limit(32 * 1024 * 1024)
            .content_type_by_string(Some(STAR_STAR))
            .unwrap()],
        ..MultipartFormDataOptions::default()
    };

    let mut multipart_form_data = match MultipartFormData::parse(content_type, data, options).await
    {
        Ok(multipart_form_data) => multipart_form_data,
        Err(err) => {
            match err {
                MultipartFormDataError::DataTooLargeError(_) => {
                    return Err("The file is too large.");
                },
                MultipartFormDataError::MulterError(multer::Error::IncompleteFieldData {
                                                        ..
                                                    })
                | MultipartFormDataError::MulterError(multer::Error::IncompleteHeaders {
                                                          ..
                                                      }) => {
                    // may happen when we set the max_data_bytes limitation
                    return Err("The request body seems too large.");
                },
                _ => panic!("{:?}", err),
            }
        },
    };

    let image = multipart_form_data.raw.remove("file");

    match image {
        Some(mut image) => {
            let raw = image.remove(0);

            let content_type = raw.content_type;
            let file_name = raw.file_name.unwrap_or_else(|| "File".to_string());
            let data = raw.raw;

            println!("File `{}` uploaded", &file_name);
            fs::write(&file_name, &data).unwrap();

            Ok(Redirect::to(uri!(index)))
        },
        None => Err("Please input a file."),
    }
}

#[main]
async fn main() {
    let local_ip = local_ip().unwrap();
    let port = 8000u16;
    let url = format!("http://{}:{}", local_ip, port);

    print_code_to_term(&make_code(&url).unwrap(), QrCodeViewArguments { margin: 2, invert_colors: false });
    println!("{}", &url);

    fs::create_dir_all("uploads").unwrap();

    rocket::build()
        .mount("/", routes![index, upload])
        .launch().await.unwrap();
}