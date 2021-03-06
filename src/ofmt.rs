use crate::utils::back_to_idx;
use crate::{mangle::Mangler, Config, Index, IndexTyp, Post};
use std::io::{Result, Write};
use std::path::Path;

const OIDXREFS_LINE_MAXLEN: usize = 100;

pub fn write_article_page<W: Write>(
    mangler: &Mangler,
    config: &Config,
    fpath: &Path,
    mut wr: W,
    rd: &Post,
    content: &str,
) -> Result<()> {
    writeln!(
        &mut wr,
        r##"<!doctype html>
<html lang="de" dir="ltr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="{}" type="text/css" />
    <title>{} &mdash; {}</title>
{}{}  </head>
  <body>
    <h1>{}</h1>
{}    <a href="#" onclick="window.history.back()">Zur&uuml;ck zur vorherigen Seite</a> - <a href="{}">Zur&uuml;ck zur Hauptseite</a>{}"##,
        config.stylesheet,
        rd.title,
        config.blog_name,
        config.x_head,
        rd.x_head,
        rd.title,
        config.x_body_ph1,
        back_to_idx(fpath),
        config.x_nav,
    )?;
    if !rd.x_nav.is_empty() {
        write!(&mut wr, " - {}", rd.x_nav)?;
    }
    write!(&mut wr, "<br />")?;
    let mut it = mangler.mangle_content(&content);
    if let Some((do_mangle, i)) = it.next() {
        if do_mangle {
            write!(&mut wr, "\n    ")
        } else {
            writeln!(&mut wr, "<br />")
        }?;
        writeln!(&mut wr, "{}", i)?;
    }
    for (do_mangle, i) in it {
        if do_mangle {
            write!(&mut wr, "    ")?;
        }
        writeln!(&mut wr, "{}", i)?;
    }
    if !rd.author.is_empty() {
        writeln!(&mut wr, "    <p>Autor: {}</p>", rd.author)?;
    }
    writeln!(&mut wr, "  </body>\n</html>")?;
    wr.flush()?;
    Ok(())
}

pub fn write_index(
    config: &Config,
    outdir: &Path,
    idx_name: &Path,
    data: &Index,
) -> std::io::Result<()> {
    println!("- index: {}", idx_name.display());

    let mut fpath = Path::new(outdir).join(idx_name);
    let (it_pre, up) = match data.typ {
        IndexTyp::Directory => {
            fpath = fpath.join("index.html");
            if idx_name.to_str().map(|i| i.is_empty()).unwrap_or(false) {
                ("", "")
            } else {
                ("Ordner: ", "<a href=\"..\">[Übergeordneter Ordner]</a>")
            }
        }
        IndexTyp::Tag => {
            fpath.set_extension("html");
            ("Tag: ", "<a href=\"index.html\">[Hauptseite]</a>")
        }
    };
    let it_post = if it_pre.is_empty() { "" } else { " &mdash; " };

    let mut f = std::io::BufWriter::new(std::fs::File::create(fpath)?);
    let idx_name_s = idx_name.to_str().unwrap();

    write!(
        &mut f,
        r#"<!doctype html>
<html lang="de" dir="ltr">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="{}" type="text/css" />
{}
    <title>{}{}{}{}</title>
{}  </head>
  <body>
    <h1>{}{}{}{}</h1>
{}
<tt>
"#,
        &config.stylesheet,
        if it_pre.is_empty() {
            r#"    <link rel="alternate" type="application/atom+xml" title="Atom feed" href="feed.atom" />
"#
        } else {
            ""
        },
        it_pre,
        idx_name_s,
        it_post,
        &config.blog_name,
        &config.x_head,
        it_pre,
        idx_name_s,
        it_post,
        &config.blog_name,
        &config.x_body_ph1,
    )?;

    if !up.is_empty() {
        writeln!(&mut f, "{}<br />", up)?;
    }

    let mut refline = String::new();
    let mut refline_len = 0;

    for i in data.oidxrefs.iter().rev() {
        let il = i.name.len();
        if (refline_len + il + 3) > OIDXREFS_LINE_MAXLEN {
            writeln!(&mut f, "{}<br />", refline)?;
            refline.clear();
            refline_len = 0;
        }
        if !refline.is_empty() {
            refline += " - ";
            refline_len += 3;
        }
        refline += &format!(
            "<a href=\"{}{}.html\">{}</a>",
            i.name.replace('&', "&amp;"),
            if i.typ == IndexTyp::Directory {
                "/index"
            } else {
                ""
            },
            i.name
        );
        refline_len += il;
    }
    if !refline.is_empty() {
        writeln!(&mut f, "{}<br />", refline)?;
        std::mem::drop(refline);
    }

    for i in data.ents.iter().rev() {
        write!(
            &mut f,
            "{}: <a href=\"{}\">{}</a>",
            i.cdate.format("%d.%m.%Y"),
            i.href,
            i.title
        )?;
        if !i.author.is_empty() {
            write!(&mut f, " <span class=\"authorspec\">by {}</span>", i.author)?;
        }
        writeln!(&mut f, "<br />")?;
    }

    writeln!(&mut f, "</tt>\n  </body>\n</html>")?;

    f.flush()?;
    f.into_inner()?.sync_all()?;
    Ok(())
}

pub fn write_feed(config: &Config, outdir: &Path, data: &Index) -> std::io::Result<()> {
    use atom_syndication::{Entry, Link, Person};
    use chrono::{DateTime, Utc};

    assert_eq!(data.typ, IndexTyp::Directory);
    println!("- atom feed");

    let now: DateTime<Utc> = Utc::now();
    let nult = chrono::NaiveTime::from_hms(0, 0, 0);

    let mut feed = atom_syndication::Feed::default();
    feed.authors = vec![{
        let mut p = Person::default();
        p.set_name(&config.author);
        p
    }];

    feed.links = vec![
        {
            let mut l = Link::default();
            l.href = config.id.clone();
            l.set_rel("alternate");
            l
        },
        {
            let mut l = Link::default();
            l.href = format!("{}/feed.atom", config.id);
            l.set_rel("self");
            l
        },
    ];

    feed.title = config.blog_name.clone();
    feed.id = config.id.clone();
    feed.set_updated(now);

    feed.entries = data
        .ents
        .iter()
        .rev()
        .take(20)
        .map(|i| {
            let mut e = Entry::default();
            e.title = if crate::utils::needs_html_escape(&i.title) {
                format!("<![CDATA[ {} ]]>", i.title)
            } else {
                i.title.clone()
            };
            e.id = i.href.clone();
            e.links = vec![{
                let mut l = Link::default();
                l.href = i.href.clone();
                l.set_rel("alternate");
                l
            }];

            let (url, updts) = if i.href.starts_with('/') || i.href.contains("://") {
                // absolute link, use cdate as update timestamp
                (
                    if i.href.starts_with('/') {
                        format!("{}{}", config.web_root_url, i.href)
                    } else {
                        i.href.clone()
                    },
                    DateTime::from_utc(i.cdate.clone().and_time(nult.clone()), Utc),
                )
            } else {
                // relative link, use mtime, or use cdate as fallback
                (
                    format!("{}/{}", config.id, i.href),
                    match std::fs::metadata(outdir.join(&i.href)) {
                        Ok(x) => crate::utils::system_time_to_date_time(x.modified().unwrap()),
                        Err(e) => {
                            eprintln!(
                                "  warning: unable to get mtime of: {}, error = {}",
                                i.href, e
                            );
                            DateTime::from_utc(i.cdate.clone().and_time(nult.clone()), Utc)
                        }
                    },
                )
            };
            e.id = url;
            e.set_updated(updts);

            e.authors = i
                .authors
                .iter()
                .map(|a| Person {
                    name: a.clone(),
                    email: None,
                    uri: None,
                })
                .collect();

            e
        })
        .collect();

    let fpath = outdir.join("feed.atom");
    let f = std::io::BufWriter::new(std::fs::File::create(fpath)?);
    let mut f = feed.write_to(f).expect("unable to serialize atom feed");
    f.flush()?;
    f.into_inner()?.sync_all()?;

    Ok(())
}
