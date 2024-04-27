use actix_multipart::Multipart;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use futures_util::{StreamExt, TryStreamExt};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::io;
use std::io::Read;
use actix_web::web::Payload;
use actix_cors::Cors;

#[derive( Debug)]
struct UploadInfo {
    filename: String,
    chunk_index: usize,
    total_chunk: usize,
}

async fn upload_chunk(mut payload: Multipart, file_path: String, info: &UploadInfo) -> Result<Option<bool>, io::Error> {
    let mut chunk_count = 0;
    while let Ok(Some(mut field)) = payload.try_next().await {
        let filepath = format!("{}/{}_{}.part", file_path, chunk_count, info.filename);
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filepath)?;

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f.write_all(&data)?;
        }
        chunk_count += 1;
    }
    Ok(Some(true))
}

async fn finalize_upload(filename: &str, upload_dir: &str, info: &UploadInfo) -> Result<(), Box<dyn std::error::Error>> {
    let mut final_path = PathBuf::from(upload_dir);
    final_path.push(filename);

    let mut final_file = File::create(final_path)
        .map_err(|err|  format!("Error creating final file: {}", err))?;

    let mut chunk_num = 0;
    while std::path::Path::new(&format!("{}/{}_{}.part", upload_dir, chunk_num, info.filename)).exists() {
        let chunk_path = format!("{}/{}_{}.part", upload_dir, chunk_num, info.filename);
        let mut chunk_file = File::open(chunk_path.clone())
            .map_err(|err| format!("Error opening chunk {}: {}", chunk_num, err))?;
        let mut chunk_data = Vec::new();
        chunk_file.read_to_end(&mut chunk_data)
            .map_err(|err| format!("Error reading chunk {}: {}", chunk_num, err))?;

        final_file.write_all(&chunk_data)
            .map_err(|err| format!("Error writing chunk {} data: {}", chunk_num, err))?;

        chunk_num += 1;
    }
    
    Ok(())
}

async fn upload(req: HttpRequest, payload: Payload) -> HttpResponse {
    let mut info = UploadInfo {
        filename: String::new(),
        chunk_index: 0,
        total_chunk: 0
    };

    let headers = req.headers();
    if let Some(header) = headers.get("X-File-Name") {
        info.filename = header.to_str().unwrap().to_string();
    }
    if let Some(header) = headers.get("X-Chunk-Index") {
        info.chunk_index = header.to_str().unwrap().parse::<usize>().unwrap();
    }
    if let Some(header) = headers.get("X-Total-Chunks") {
        info.total_chunk = header.to_str().unwrap().parse::<usize>().unwrap();
    }

    let upload_dir = "uploads";

        if !std::path::Path::new(upload_dir).exists() {
            match std::fs::create_dir_all(upload_dir) {
                Ok(_) => println!("Upload directory created successfully"),
                Err(err) => return HttpResponse::InternalServerError().body(format!("Error creating upload directory: {}", err)),
            }
        }

    let multipart = Multipart::new(headers, payload);

    if let Ok(Some(upload_result)) = upload_chunk(multipart, upload_dir.to_string(), &info).await {
        if upload_result {
            if let Err(err) = finalize_upload(&info.filename, upload_dir, &info).await {
                return HttpResponse::InternalServerError().body(err.to_string());
            }
            if info.chunk_index == info.total_chunk {
                match std::fs::remove_file(format!("{}/{}_{}.part", upload_dir, 0, info.filename)) {
                    Ok(_) => println!("Temporary part file removed successfully"),
                    Err(err) => println!("Error removing temporary part: {}", err),
                }
            }
            return HttpResponse::Ok().body(format!("{} - chunk {} have been succeeded upload",info.filename, info.chunk_index));
        } else {
            return HttpResponse::InternalServerError().body("Error during upload");
        }
    } else {
        return HttpResponse::InternalServerError().body("Error processing upload");
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let cors = Cors::default()
        .allowed_origin("http://localhost:4200")
        .allowed_methods(vec!["POST"])
        .allowed_headers(vec!["Content-Type","X-File-Name", "X-File-Size", "X-Chunk-Index", "X-Total-Chunks"])
        .max_age(3600);

        App::new()
            .app_data(web::Data::new(String::from("uploads")))
            .wrap(cors)
            .route("/upload", web::post().to(upload))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await  
}

