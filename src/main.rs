extern crate select;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::env;
use std::io::{self, Write, Cursor};
use std::fs::File;
use std::path::Path;

use select::document::Document;
use select::predicate::Attr;

use futures::Future;
use futures::stream::Stream;

use hyper::Client;

fn main() {
    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: client <url>");
            return;
        }
    };

    let url = url.parse::<hyper::Uri>().unwrap();
    if url.scheme() != Some("http") {
        println!("This example only works with 'http' URLs.");
        return;
    }

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);
    let mut files: Vec<File> = Vec::new();

    let work = client.get(url).and_then(|res| {
        res.body().collect().map(|chunks| {
            let mut data = Vec::new();
            for chunk in chunks {
                data.extend(chunk);
            }
            let document = Document::from_read(Cursor::new(data)).unwrap();
            let mut i = 0usize;
            let mut works = Vec::new();
            for photos in document.find(Attr("id", "photoList")) {
                for photo in photos.children() {
                    if let Some(Ok(url)) = photo.attr("href").map(|url| url.parse::<hyper::Uri>()) {
                        unsafe {
                            // Only add at end
                            (&mut *(&files as *const _ as usize as *mut Vec<File>)).push(File::create(format!("img{}.jpg", i)).unwrap());
                        }
                        let work = client.get(url).map(move |res| (i, res)).and_then(|(i, res)| {
                            res.body().map(move |res| (i, res)).for_each(|(i, chunk)| {
                                    unsafe {
                                        // Only access one already added item
                                        (&mut *(&files[i] as *const _ as usize as *mut File)).write_all(&chunk).map_err(From::from)
                                    }
                            })
                        });
                        works.push(work);
                        i+=1;
                    }
                }
            }
            works
        })
    });

    let works = core.run(work).unwrap();
    for work in works {
        core.run(work);
    } 
}
