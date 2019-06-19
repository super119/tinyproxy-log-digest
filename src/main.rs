#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate actix_web;
extern crate askama;
extern crate mktemp;
extern crate flate2;

mod errors;
mod utils;

use errors::*;
use log::LevelFilter;
use std::fs;
use std::io::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{web, App, HttpResponse, HttpServer};
use askama::Template;
use mktemp::Temp;
use flate2::read::GzDecoder;
use utils::*;

const TINYPROXY_LOG_DIR:&'static str = "/var/log/tinyproxy";
const ISE:&'static str = "Parsing tinyproxy log failed";
const HTTP_GET:&'static str = "GET";
const HTTP_POST:&'static str = "POST";

struct Record {
    // timestamp
    ts: u64,
    // human friendly timestamp
    hts: String,
    method: String,
    url: String,
    url_shrink: String,
    counts: usize,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    records: Vec<Record>
}

fn save_record(records: &mut Vec<Record>, s: Record) {
    let mut found = false;
    let mut i = 0;
    while i < records.len() {
        let r = &mut records[i];
        if r.method == s.method && r.url == s.url {
            r.counts += 1;
            if r.ts < s.ts {
                r.ts = s.ts;
                r.hts = s.hts.to_string();
            }
            found = true;
            break;
        }
        i += 1;
    }

    if !found {
        records.push(s);
    }
}

fn parse_logs() -> Result<Vec<Record>> {
    let mut records: Vec<Record> = Vec::new();

    let td = Temp::new_dir().chain_err(|| "Create temp dir failed.")?;
    let tdp = td.to_path_buf();
    info!("Temp directory is created: {:?}", tdp);

    let mut num = 0;
    let entries = fs::read_dir(TINYPROXY_LOG_DIR)
                    .chain_err(|| format!("Read tinyproxy log dir: {} failed.", TINYPROXY_LOG_DIR))?;
    for entry in entries {
        let dir_entry = match entry {
            Ok(o) => o,
            Err(e) => {
                warn!("Iterating tinyproxy log dir: {} failed, ignore and continue. Reason: {}",
                      TINYPROXY_LOG_DIR, e);
                continue;
            }
        };
        let path = dir_entry.path();
        if path.is_dir() {
            continue;
        }
        info!("Got one tinyproxy log: {:?}", path);
        let zip: bool;
        if let Some(f) = path.to_str() {
            zip = f.ends_with(".gz");
        } else {
            warn!("Get tinyproxy log file path failed, ignored.");
            continue;
        }

        num += 1;
        let mut log = tdp.clone();
        if zip {
            log.push(format!("log{}.gz", num));
        } else {
            log.push(format!("log{}", num));
        }
        info!("Copying {:?} to {:?}", path, log);
        fs::copy(path.as_path(), log.as_path())?;

        // Be noticed that below we don't use read_to_string, because this
        // function converts bytes to String using encoding UTF8.
        // For some files which contain invalid UTF8 characters, this function
        // fails. So using from_utf8_lossy instead here.
        let mut s_log: String;
        if zip {
            info!("Uncompressing log: {:?}", log);
            let mut f = fs::File::open(log.as_path())?;
            let mut buf: Vec<u8> = Vec::new();
            f.read_to_end(&mut buf)?;
            let mut d = GzDecoder::new(&buf[..]);
            let mut sbuf: Vec<u8> = Vec::new();
            d.read_to_end(&mut sbuf)?;
            s_log = String::from_utf8_lossy(&sbuf).to_string();
        } else {
            let mut f = fs::File::open(log.as_path())?;
            let mut sbuf: Vec<u8> = Vec::new();
            f.read_to_end(&mut sbuf)?;
            s_log = String::from_utf8_lossy(&sbuf).to_string();
        }

        for l in s_log.lines() {
            let line = l.trim().to_string();
            if line.len() == 0 {
                continue;
            }
            let get = line.find(HTTP_GET);
            let post = line.find(HTTP_POST);
            if get == None && post == None {
                continue;
            }

            // Get timestamp
            let mut toks: Vec<String> = Vec::new();
            let splits = line.split(' ');
            for s in splits {
                if s.trim().len() > 0 {
                    toks.push(s.to_string());
                }
            }
            let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let year = timestamp_get_year(elapsed);
            let ts = match date_time_to_timestamp(format!("{} {} {} {}",
                                                  toks[1], toks[2], toks[3], year)) {
                Ok(o) => o,
                Err(e) => {
                    warn!("Parse timestamp in log failed: {}, ignored.", e);
                    continue;
                }
            };

            if get != None {
                let pos = get.unwrap();
                let ss = &line[(pos + 4)..];
                let end = match ss.find(' ') {
                    Some(o) => o,
                    None => {
                        warn!("Can't get the URL of method GET, ignored. Log: {}", ss);
                        continue;
                    }
                };
                let url = &ss[..end];
                let url_short;
                if url.len() > 100 {
                    url_short = format!("{}...{}", &url[..50], &url[(url.len() - 50)..]);
                } else {
                    url_short = url.to_string();
                }

                let rec = Record {
                    ts: ts,
                    hts: timestamp_to_date_time(ts),
                    method: HTTP_GET.to_string(),
                    url: url.to_string(),
                    url_shrink: url_short,
                    counts: 1,
                };
                save_record(&mut records, rec);
            }

            if post != None {
                let pos = post.unwrap();
                let ss = &line[(pos + 5)..];
                let end = match ss.find(' ') {
                    Some(o) => o,
                    None => {
                        warn!("Can't get the URL of method POST, ignored. Log: {}", ss);
                        continue;
                    }
                };
                let url = &ss[..end];
                let url_short;
                if url.len() > 100 {
                    url_short = format!("{}...{}", &url[..50], &url[(url.len() - 50)..]);
                } else {
                    url_short = url.to_string();
                }

                let rec = Record {
                    ts: ts,
                    hts: timestamp_to_date_time(ts),
                    method: HTTP_POST.to_string(),
                    url: url.to_string(),
                    url_shrink: url_short,
                    counts: 1,
                };
                save_record(&mut records, rec);
            }
        }
    }

    records.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(records)
}

fn index() -> actix_web::Result<HttpResponse> {
    info!("Index page request is received. Start parsing tinyproxy log.");
    let recs = match parse_logs() {
        Ok(o) => o,
        Err(e) => {
            error!("Parsing tinyproxy log failed: {}", e);
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body(ISE));
        }
    };

    let i = Index {
        records: recs
    }.render().unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(i))
}

fn main() -> std::io::Result<()> {
    env_logger::init();
    log::set_max_level(LevelFilter::Debug);

    info!("tinyproxy-log-parse starts...");
    HttpServer::new(move || {
        App::new().service(web::resource("/").route(web::get().to(index)))
    }).bind("0.0.0.0:8080").unwrap().run()
}
