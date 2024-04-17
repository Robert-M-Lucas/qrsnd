#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::{Config, Data};
use std::fs;
use local_ip_address::local_ip;
use qrrs::qrcode::{make_code, print_code_to_term, QrCodeViewArguments};
use rocket::data::{Limits, ToByteUnit};
use rocket::http::ContentType;
use rocket::log::LogLevel;
use rocket::response::content::RawHtml;
use rocket_multipart_form_data::{multer, MultipartFormData, MultipartFormDataError, MultipartFormDataField, MultipartFormDataOptions};
use rocket_multipart_form_data::multer::bytes::buf::Limit;
use rocket_raw_response::mime::{STAR_STAR};

const INDEX_HTML: &'static str = include_str!("index.html");

#[get("/")]
fn index() -> RawHtml<String> {
    RawHtml(INDEX_HTML.to_string().replace("$$$$", "Click to Upload"))
}

#[post("/upload", data = "<data>")]
async fn upload(content_type: &ContentType, data: Data<'_>) -> Result<RawHtml<String>, &'static str> {
    let options = MultipartFormDataOptions {
        max_data_bytes: 10.gibibytes().as_u64(),
        allowed_fields: vec![MultipartFormDataField::raw("file")
            .size_limit(10.gibibytes().as_u64())
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

            let _content_type = raw.content_type;
            let file_name = raw.file_name.unwrap_or_else(|| "File".to_string());
            let data = raw.raw;

            println!("File `{}` uploaded", &file_name);
            fs::write(&file_name, &data).unwrap();

            Ok(RawHtml(INDEX_HTML.to_string().replace("$$$$", &format!("{} uploaded<br>Click to Upload", &file_name))))
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
    println!("This is an unsecured connection. Don't send sensitive information - don't use on public WiFi!");

    let config = Config {
        address: "0.0.0.0".parse().unwrap(),
        log_level: LogLevel::Off,
        limits: Limits::new()
            .limit("form", 10.gibibytes())
            .limit("data-form", 10.gibibytes())
            .limit("file", 10.gibibytes())
            .limit("string", 10.gibibytes())
            .limit("bytes", 10.gibibytes())
            .limit("json", 10.gibibytes())
            .limit("msgpack", 10.gibibytes()),
        ..Config::release_default()
    };

    rocket::custom(config)
        .mount("/", routes![index, upload])
        .launch().await.unwrap();
}