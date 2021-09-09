mod cli;
mod parsing;


use std::convert::TryInto;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};

use clap::Clap;
use env_logger;
use xml::reader::{EventReader, XmlEvent};

use crate::cli::Opts;
use crate::parsing::xhtml_to_plain;


fn talk_to_server(server_port: u16, title: &str, wikitext: &str) -> Result<String, io::Error> {
    // open socket
    let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), server_port))?;

    // write magic
    stream.write_all(b"WiKiCrUnCh")?;

    // write title length
    let title_len_i32: i32 = title.len()
        .try_into().expect("failed to store title length as 32-bit integer");
    let title_len_bytes = title_len_i32.to_be_bytes();
    stream.write_all(&title_len_bytes)?;

    // write title
    stream.write_all(title.as_bytes())?;

    // write length
    let wikitext_len_i32: i32 = wikitext.len()
        .try_into().expect("failed to store wikitext length as 32-bit integer");
    let wikitext_len_bytes = wikitext_len_i32.to_be_bytes();
    stream.write_all(&wikitext_len_bytes)?;

    // write text
    stream.write_all(wikitext.as_bytes())?;

    // read length
    let mut html_len_bytes = [0u8; 4];
    stream.read_exact(&mut html_len_bytes)?;
    let html_len_i32 = i32::from_be_bytes(
        html_len_bytes.try_into().expect("failed to store HTML length as 32-bit integer")
    );
    let html_len_usize: usize = html_len_i32.try_into().expect("failed to store HTML length as usize");

    // read as many bytes
    let mut html_bytes = vec![0u8; html_len_usize];
    stream.read_exact(&mut html_bytes)?;

    // try to turn it into a string
    let string = String::from_utf8(html_bytes)
        .expect("failed to parse HTML as UTF-8");

    // send our goodbyes
    stream.write_all(b"EnOuGhWiKi")?;

    Ok(string)
}


macro_rules! output_line {
    ($file:expr, $($args:expr),*) => {
        if let Some(f) = &mut $file {
            writeln!(f, $($args),*).expect("failed to write");
        } else {
            println!($($args),*);
        }
    };
}


fn main() {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let page_level = vec![
        "mediawiki", "page",
    ];
    let title_level = vec![
        "mediawiki", "page", "title",
    ];
    let text_level = vec![
        "mediawiki", "page", "revision", "text",
    ];

    {
        let file = File::open(&opts.wiki_xml).unwrap();
        let mut out_file = opts.output_file.map(|f| File::create(f).expect("failed to open output file"));
        let reader = BufReader::new(file);

        let parser = EventReader::new(reader);
        let mut element_stack: Vec<String> = Vec::new();

        let mut page_count: usize = 0;
        let mut text = String::new();
        let mut store_text = false;
        let mut current_title: Option<String> = None;
        let mut keep_going = false;

        for event_res in parser {
            let event = event_res.unwrap();

            match event {
                XmlEvent::StartElement { name, .. } => {
                    element_stack.push(name.local_name.clone());
                    if element_stack == page_level {
                        page_count += 1;
                        current_title = None;
                    } else if element_stack == text_level || element_stack == title_level {
                        text.clear();
                        store_text = true;
                    }
                },
                XmlEvent::EndElement { .. } => {
                    if store_text {
                        store_text = false;

                        if element_stack == text_level {
                            let mut parse_it = true;

                            if let Some(ct) = &current_title {
                                if ct.starts_with("Medium:") || ct.starts_with("Spezial:")
                                    || ct.starts_with("Diskussion:")
                                    || ct.starts_with("Benutzer:") || ct.starts_with("Benutzer Diskussion:")
                                    || ct.starts_with("Datei:") || ct.starts_with("Datei Diskussion:")
                                    || ct.starts_with("Vorlage:") || ct.starts_with("Vorlage Diskussion:")
                                    || ct.starts_with("Kategorie:") || ct.starts_with("Kategorie Diskussion:") {

                                    parse_it = false;
                                }
                            }

                            if parse_it && !keep_going {
                                if let Some(ot) = &opts.title {
                                    if let Some(ct) = &current_title {
                                        parse_it = ot == ct;
                                    } else {
                                        parse_it = false;
                                    }
                                } else {
                                    parse_it = true;
                                };
                            }

                            if parse_it {
                                let page_title = current_title
                                    .as_deref()
                                    .unwrap_or("Unbekannte Seite");
                                let xhtml = talk_to_server(opts.parse_server_port, page_title, &text)
                                    .unwrap();

                                if xhtml.len() > 0 {
                                    if opts.xhtml_output {
                                        output_line!(out_file, "{}", xhtml);
                                    }

                                    if !opts.no_plain_output {
                                        let plaintext = xhtml_to_plain(&xhtml)
                                            .unwrap();
                                        output_line!(out_file, "{}", plaintext);
                                    }
                                }

                                if opts.and_after {
                                    keep_going = true;
                                } else if opts.title.is_some() {
                                    break;
                                }
                            }
                        } else {
                            assert_eq!(element_stack, title_level);
                            // title
                            let output_title = if let Some(t) = &opts.title {
                                if keep_going {
                                    true
                                } else if let Some(ct) = &current_title {
                                    ct == t
                                } else {
                                    false
                                }
                            } else {
                                true
                            };
                            if output_title {
                                output_line!(out_file, "# {} TITLE: {}", page_count, text);
                            }
                            current_title = Some(text.clone());
                        }
                    }
                    element_stack.pop();
                },
                XmlEvent::Characters(chars) => {
                    if store_text {
                        text.push_str(&chars);
                    }
                },
                XmlEvent::CData(_) => {
                    panic!("CDATA!");
                },
                _ => {},
            }
        }
    }
}
