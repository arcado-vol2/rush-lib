use crate::config::{Config, DocOut, Theme};
use crate::content::Content;
use crate::parse::block_fragmentation::Counters;
use crate::parse::rush::Rush;
use crate::project::Project;
use crate::resources::Resources;
use crate::style::{PageGeometry, Style};
use crate::OutFile;
use std::collections::BTreeSet;
use std::fmt::{self, Write};
use std::path::Path;

pub(crate) mod fixed;
pub(crate) mod ir;
pub(crate) mod semantic;
mod warning;

pub use warning::{Feature, Origin, RenderWarning, RenderWarningKind};

pub(crate) const FONTS_DIR: &str = "fonts";

pub(crate) trait Semantic {
    fn out(&self) -> DocOut;
    fn supports(&self, feature: Feature) -> bool;
    fn ext(&self) -> &'static str;
    fn assets(&self, style: &Style) -> Vec<OutFile>;
    fn page(&self, doc: &ir::Doc, ctx: &semantic::PageCtx) -> Result<String, fmt::Error>;
    /// The whole project as one file
    fn joined(&self, project: &ir::Project, ctx: &semantic::PageCtx) -> Result<String, fmt::Error>;
}

pub(crate) trait Fixed {
    fn out(&self) -> DocOut;
    fn supports(&self, feature: Feature) -> bool;
    fn paint(&self, layout: &fixed::Layout, style: &Style) -> Vec<OutFile>;
}

pub(crate) enum Backend {
    Semantic(&'static dyn Semantic),
    Fixed(&'static dyn Fixed),
}

impl Backend {
    fn of(config: &Config) -> Backend {
        match config.out {
            DocOut::Html if config.single => Backend::Fixed(&fixed::html_paged::HtmlPaged),
            DocOut::Html => Backend::Semantic(&semantic::html::Html),
            DocOut::Markdown => Backend::Semantic(&semantic::markdown::Markdown),
            _ => unimplemented!("not yet"),
        }
    }

    fn out(&self) -> DocOut {
        match self {
            Backend::Semantic(b) => b.out(),
            Backend::Fixed(b) => b.out(),
        }
    }

    fn supports(&self, feature: Feature) -> bool {
        match self {
            Backend::Semantic(b) => b.supports(feature),
            Backend::Fixed(b) => b.supports(feature),
        }
    }
}

pub(crate) fn escape<W: Write + ?Sized>(out: &mut W, s: &str) -> fmt::Result {
    let mut last = 0;
    for (i, c) in s.char_indices() {
        let rep = match c {
            '<' => "&lt;",
            '>' => "&gt;",
            '&' => "&amp;",
            '"' => "&quot;",
            _ => continue
        };
        out.write_str(&s[last..i])?;
        out.write_str(rep)?;
        last = i + 1;
    }
    out.write_str(&s[last..])
}

pub(crate) fn css_fonts(style: &Style, dir: &str) -> String {
    let mut out = String::new();
    for (family, font, (file, _)) in style.fonts.all() {
        let _ = writeln!(out, "@font-face {{ font-family: \"{}\"; src: url(\"{dir}/{file}\"); font-weight: {}; font-style: {}; }}",
            family.name,
            if font.bold { "bold" } else { "normal" },
            if font.italic { "italic" } else { "normal" });
    }
    out
}

fn font_files(style: &Style) -> Vec<OutFile> {
    style.fonts.all().map(|(_, _, (name, data))| OutFile { path: format!("{FONTS_DIR}/{name}"), data: data.to_vec() }).collect()
}

fn check(used: &BTreeSet<Feature>, file: &str, style: &Style, backend: &Backend, warn: &mut Vec<RenderWarning>) {
    for &f in used {
        if !style.supports(f) {
            warn.push(RenderWarning { origin: Origin::Style(style.name), file: Some(file.to_string()), kind: RenderWarningKind::Unsupported(f) });
        }
        if !backend.supports(f) {
            warn.push(RenderWarning { origin: Origin::Backend(backend.out()), file: Some(file.to_string()), kind: RenderWarningKind::Unsupported(f) });
        }
    }
}

fn out_path(local: &Path) -> String {
    local.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

pub(crate) fn geometry(style: &Style, warn: &mut Vec<RenderWarning>) -> PageGeometry {
    style.page.unwrap_or_else(|| {
        warn.push(RenderWarning { origin: Origin::Style(style.name), file: None, kind: RenderWarningKind::Unsupported(Feature::Paging) });
        PageGeometry::A4
    })
}

/// Content files in order, numbered across all of them
fn content_sources<'a>(content: &'a Content, res: &'a Resources) -> Vec<ir::Source<'a>> {
    let mut counters = Counters::default();
    let mut sources = Vec::with_capacity(content.items.len());
    for item in &content.items {
        let rush = Rush::new_from(res.files.data(item.file), res, counters);
        counters = rush.counters;
        sources.push(ir::Source { file: item.file, title: &item.heading, rush });
    }
    sources
}

fn theme(config: &Config, style: &Style, backend: &Backend, warn: &mut Vec<RenderWarning>) -> Theme {
    let unsupported = |origin, warn: &mut Vec<RenderWarning>| {
        warn.push(RenderWarning { origin, file: None, kind: RenderWarningKind::UnsupportedTheme(config.theme) });
        Theme::Light
    };
    match (config.theme, backend) {
        (Theme::Light, _) => Theme::Light,
        (Theme::Auto, Backend::Semantic(_)) if config.out == DocOut::Markdown || style.dark.is_none() => Theme::Light,
        (Theme::Dark, _) if config.out == DocOut::Markdown => unsupported(Origin::Backend(config.out), warn),
        (Theme::Dark, _) if style.dark.is_none() => unsupported(Origin::Style(style.name), warn),
        (Theme::Auto, Backend::Fixed(_)) if style.dark.is_none() => Theme::Light,
        (Theme::Auto, Backend::Fixed(b)) => unsupported(Origin::Backend(b.out()), warn),
        (theme, _) => theme,
    }
}

fn check_all(ir: &ir::Project, res: &Resources, style: &Style, backend: &Backend, warn: &mut Vec<RenderWarning>) {
    for doc in &ir.docs {
        check(&doc.used, &res.files.local_path(doc.file).to_string_lossy(), style, backend, warn);
    }
}

pub(crate) fn build(config: &Config, project: &Project, content: &Content) -> Result<(Vec<OutFile>, Vec<RenderWarning>), fmt::Error> {
    let mut style = Style::of(config.style);
    if config.out == DocOut::Markdown {
        style.non_russian = None;
    }
    let backend = Backend::of(config);
    let res = &project.resources;
    let mut warn = Vec::new();
    let theme = theme(config, &style, &backend, &mut warn);
    let scripts = !config.no_js;
    let mut files = font_files(&style);
    match &backend {
        Backend::Semantic(b) if config.single => {
            files.extend(b.assets(&style));
            let ctx = semantic::PageCtx { res, content, style: &style, theme, scripts };
            let ir = ir::build(content_sources(content, res), &style, res, &mut warn);
            check_all(&ir, res, &style, &backend, &mut warn);
            files.push(OutFile { path: format!("index.{}", b.ext()), data: b.joined(&ir, &ctx)?.into_bytes() });
        }
        Backend::Semantic(b) => {
            files.extend(b.assets(&style));
            let ctx = semantic::PageCtx { res, content, style: &style, theme, scripts };
            for (id, local, text) in res.files.iter() {
                let stem = local.file_stem().unwrap_or_default().to_string_lossy();
                let title = content.title_for(id).unwrap_or(&stem);
                let src = ir::Source { file: id, title, rush: Rush::new(text, res) };
                let doc = ir::build(vec![src], &style, res, &mut warn).docs.remove(0);
                check(&doc.used, &local.to_string_lossy(), &style, &backend, &mut warn);
                let page = b.page(&doc, &ctx)?;
                files.push(OutFile { path: out_path(local.with_extension(b.ext()).as_path()), data: page.into_bytes() });
            }
        }
        Backend::Fixed(b) => {
            if let (Theme::Dark, Some(dark)) = (theme, style.dark) {
                style = Style { non_russian: style.non_russian, ..dark() };
            }
            let geometry = geometry(&style, &mut warn);
            let ir = ir::build(content_sources(content, res), &style, res, &mut warn);
            check_all(&ir, res, &style, &backend, &mut warn);
            let mut w = fixed::flow::Warn { out: &mut warn, backend: b.out() };
            let layout = fixed::layout(&ir, &style, geometry, res, &mut w);
            files.extend(b.paint(&layout, &style));
        }
    }
    Ok((files, warn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DocStyle;

    fn project() -> (Project, Content) {
        let mut p = Project::new();
        p.add_file("a", "a.rush", "\\++ Раздел\n\nтекст\\*1\n\n\\**1 сноска".into()).unwrap();
        p.add_content("Документ".into(), "a".into());
        let (content, _) = Content::resolve(&p.content, &p.dirs, &p.resources);
        (p, content)
    }

    fn paths(files: &[OutFile]) -> Vec<&str> {
        files.iter().map(|f| f.path.as_str()).filter(|p| !p.starts_with(FONTS_DIR)).collect()
    }

    #[test]
    fn same_style_two_backends() {
        let (p, content) = project();
        let html = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html, single: false, ..Config::default() }, &p, &content).unwrap();
        assert_eq!(paths(&html.0), vec!["style.css", "a.html"]);
        let paged = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html, single: true, ..Config::default() }, &p, &content).unwrap();
        assert_eq!(paths(&paged.0), vec!["style.css", "index.html"]);
        assert!(html.1.is_empty() && paged.1.is_empty(), "{:?} {:?}", html.1, paged.1);

        let page = String::from_utf8(html.0[html.0.len() - 1].data.clone()).unwrap();
        assert!(page.contains("<h2 id=\"h-1-1\"><span class=\"index\">1</span> Раздел</h2>"), "{page}");
        let css = String::from_utf8(paged.0.iter().find(|f| f.path == "style.css").unwrap().data.clone()).unwrap();
        assert!(!css.contains("display: none"), "стиль не прячет и не переставляет");
    }

    fn text_of<'f>(files: &'f [OutFile], path: &str) -> &'f str {
        std::str::from_utf8(&files.iter().find(|f| f.path == path).unwrap().data).unwrap()
    }

    fn two_docs() -> (Project, Content) {
        let mut p = Project::new();
        p.add_image("pic", "img/pic.png", Some((10, 10))).unwrap();
        p.add_file("b/one", "b/one.rush", "\\++ Раздел Rust\n\nтекст\\*1\n\n\\**1 сноска\n\n\\,two(ссылка)\n\n\\(pic.png, Схема)\n".into()).unwrap();
        p.add_file("two", "two.rush", "\\++ Второй\n\nещё\\*1\n\n\\**1 другая".into()).unwrap();
        p.add_content("Второй файл".into(), "two".into());
        p.add_content("Первый файл".into(), "b/one".into());
        let (content, _) = Content::resolve(&p.content, &p.dirs, &p.resources);
        (p, content)
    }

    #[test]
    fn markdown_single_is_one_file_in_content_order() {
        let (p, content) = two_docs();
        let config = Config { style: DocStyle::Gost7_32, out: DocOut::Markdown, single: true, ..Config::default() };
        let (files, _) = build(&config, &p, &content).unwrap();
        assert_eq!(paths(&files), vec!["index.md"]);
        let md = text_of(&files, "index.md");
        let (second, first) = (md.find("# Второй файл").unwrap(), md.find("# b; Первый файл").unwrap());
        assert!(second < first, "{md}");
        assert!(md.contains("[^1]: другая") && md.contains("[^2]: сноска"), "{md}");
        assert!(md.contains("## 1 Второй") && md.contains("## 2 Раздел Rust"), "{md}");
        assert!(!md.contains("two.md"), "{md}");
        assert!(md.contains("](img/pic.png)"), "{md}");

        let tree = build(&Config { single: false, ..config }, &p, &content).unwrap();
        assert!(text_of(&tree.0, "b/one.md").contains("](../img/pic.png)"));
    }

    #[test]
    fn non_russian_only_outside_markdown() {
        let (p, content) = two_docs();
        let md = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Markdown, single: false, ..Config::default() }, &p, &content).unwrap();
        assert!(text_of(&md.0, "b/one.md").contains("Раздел Rust"));
        let html = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html, single: false, ..Config::default() }, &p, &content).unwrap();
        assert!(text_of(&html.0, "b/one.html").contains("<i><span class=\"mono\">Rust</span></i>"));
        let paged = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html, single: true, ..Config::default() }, &p, &content).unwrap();
        assert_eq!(text_of(&paged.0, "index.html").matches("<span class=\"mono\">Rust</span>").count(), 2);
    }

    #[test]
    fn title_tag_stays_plain() {
        let mut p = Project::new();
        p.add_file("a", "a.rush", "текст".into()).unwrap();
        p.add_content("Про Rust".into(), "a".into());
        let (content, _) = Content::resolve(&p.content, &p.dirs, &p.resources);
        let (files, _) = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html, single: false, ..Config::default() }, &p, &content).unwrap();
        let page = text_of(&files, "a.html");
        assert!(page.contains("<title>Про Rust</title>"), "{page}");
        assert!(page.contains("<header class=\"title\"><span class=\"name\">Про <i><span class=\"mono\">Rust</span></i></span></header>"), "{page}");
    }

    #[test]
    fn theme_warnings() {
        use crate::config::Theme::{Auto, Dark};
        let (p, content) = project();
        let warns = |style, out, single, theme| build(&Config { style, out, single, theme, ..Config::default() }, &p, &content).unwrap().1;
        let unsupported = |origin, theme| vec![RenderWarning { origin, file: None, kind: RenderWarningKind::UnsupportedTheme(theme) }];
        assert_eq!(warns(DocStyle::Gost7_32, DocOut::Html, false, Dark), unsupported(Origin::Style(DocStyle::Gost7_32), Dark));
        assert_eq!(warns(DocStyle::Modern, DocOut::Html, true, Dark), unsupported(Origin::Style(DocStyle::Modern), Dark));
        assert_eq!(warns(DocStyle::Blog, DocOut::Markdown, false, Dark), unsupported(Origin::Backend(DocOut::Markdown), Dark));
        assert_eq!(warns(DocStyle::Blog, DocOut::Html, true, Auto), unsupported(Origin::Backend(DocOut::Html), Auto));
        for (style, out, single, theme) in [
            (DocStyle::Gost7_32, DocOut::Html, true, Auto),
            (DocStyle::Modern, DocOut::Html, false, Auto),
            (DocStyle::Gost7_32, DocOut::Markdown, false, Auto),
            (DocStyle::Blog, DocOut::Html, false, Auto),
            (DocStyle::Blog, DocOut::Html, false, Dark),
            (DocStyle::Blog, DocOut::Html, true, Dark),
        ] {
            assert_eq!(warns(style, out, single, theme), vec![], "{style:?} {out:?} {single} {theme:?}");
        }
    }

    #[test]
    fn blog_single_dark_paints_dark_pages() {
        let (p, content) = project();
        let config = Config { style: DocStyle::Blog, out: DocOut::Html, single: true, theme: crate::config::Theme::Dark, ..Config::default() };
        let (files, _) = build(&config, &p, &content).unwrap();
        assert!(text_of(&files, "style.css").contains("--paper: #141617"));
    }

    #[test]
    fn markdown_single_heads_with_opened_dirs() {
        let mut p = Project::new();
        for path in ["a/b.rush", "a/c.rush", "x.rush"] {
            p.add_file(path, path, "текст".into()).unwrap();
        }
        for (name, key) in [("B", "a/b"), ("C", "a/c"), ("X", "x")] {
            p.add_content(name.into(), key.into());
        }
        p.add_dir("A".into(), "a".into());
        let (content, _) = Content::resolve(&p.content, &p.dirs, &p.resources);
        let config = Config { style: DocStyle::Blog, out: DocOut::Markdown, single: true, ..Config::default() };
        let (files, _) = build(&config, &p, &content).unwrap();
        let md = text_of(&files, "index.md");
        assert!(md.starts_with("# A; B

текст

# C

текст

# X"), "{md}");
    }

    #[test]
    fn style_without_pages_warns() {
        let style = Style { page: None, ..Style::of(DocStyle::Gost7_32) };
        let mut warn = Vec::new();
        assert_eq!(geometry(&style, &mut warn).width, PageGeometry::A4.width);
        assert_eq!(warn, vec![RenderWarning { origin: Origin::Style(DocStyle::Gost7_32), file: None, kind: RenderWarningKind::Unsupported(Feature::Paging) }]);
    }
}
