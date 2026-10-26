use crate::config::{Config, DocOut};
use crate::content::Content;
use crate::parse::block_fragmentation::Counters;
use crate::parse::rush::Rush;
use crate::project::Project;
use crate::style::{PageGeometry, Style};
use crate::OutFile;
use std::collections::BTreeSet;
use std::fmt::{self, Write};
use std::path::Path;

pub(crate) mod fixed;
pub(crate) mod ir;
pub(crate) mod markdown;
pub(crate) mod semantic;
mod warning;

pub use warning::{Feature, Origin, RenderWarning, RenderWarningKind};

pub(crate) const FONTS_DIR: &str = "fonts";

/// Format that writes the structure and leaves the layout to the viewer
pub(crate) trait Semantic {
    fn out(&self) -> DocOut;
    fn supports(&self, feature: Feature) -> bool;
    fn ext(&self) -> &'static str;
    fn assets(&self, style: &Style) -> Vec<OutFile>;
    fn page(&self, doc: &ir::Doc, ctx: &semantic::PageCtx) -> Result<String, fmt::Error>;
}

/// Format that draws finished pages
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
    fn of(out: DocOut) -> Backend {
        match out {
            DocOut::Html => Backend::Semantic(&semantic::html::Html),
            DocOut::HtmlPaged => Backend::Fixed(&fixed::html_paged::HtmlPaged),
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

/// `@font-face` for all faces of the style; the files are in `dir` next to the css
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

/// Page geometry of the style; A4 with a warning if the style does not set it
pub(crate) fn geometry(style: &Style, warn: &mut Vec<RenderWarning>) -> PageGeometry {
    style.page.unwrap_or_else(|| {
        warn.push(RenderWarning { origin: Origin::Style(style.name), file: None, kind: RenderWarningKind::Unsupported(Feature::Paging) });
        PageGeometry::A4
    })
}

pub(crate) fn build(config: &Config, project: &Project, content: &Content) -> Result<(Vec<OutFile>, Vec<RenderWarning>), fmt::Error> {
    let style = Style::of(config.style);
    let backend = Backend::of(config.out);
    let res = &project.resources;
    let mut warn = Vec::new();
    let mut files = font_files(&style);
    match &backend {
        Backend::Semantic(b) => {
            files.extend(b.assets(&style));
            let ctx = semantic::PageCtx { res, content, style: &style };
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
            let geometry = geometry(&style, &mut warn);
            let mut counters = Counters::default();
            let mut sources = Vec::with_capacity(content.items.len());
            for item in &content.items {
                let rush = Rush::new_from(res.files.data(item.file), res, counters);
                counters = rush.counters;
                sources.push(ir::Source { file: item.file, title: &item.name, rush });
            }
            let ir = ir::build(sources, &style, res, &mut warn);
            for doc in &ir.docs {
                check(&doc.used, &res.files.local_path(doc.file).to_string_lossy(), &style, &backend, &mut warn);
            }
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
        let (content, _) = Content::resolve(&p.content, &p.resources);
        (p, content)
    }

    fn paths(files: &[OutFile]) -> Vec<&str> {
        files.iter().map(|f| f.path.as_str()).filter(|p| !p.starts_with(FONTS_DIR)).collect()
    }

    #[test]
    fn same_style_two_backends() {
        let (p, content) = project();
        let html = build(&Config { style: DocStyle::Gost7_32, out: DocOut::Html }, &p, &content).unwrap();
        assert_eq!(paths(&html.0), vec!["style.css", "a.html"]);
        let paged = build(&Config { style: DocStyle::Gost7_32, out: DocOut::HtmlPaged }, &p, &content).unwrap();
        assert_eq!(paths(&paged.0), vec!["style.css", "index.html"]);
        assert!(html.1.is_empty() && paged.1.is_empty(), "{:?} {:?}", html.1, paged.1);

        let page = String::from_utf8(html.0[html.0.len() - 1].data.clone()).unwrap();
        assert!(page.contains("<h2 id=\"h-1-1\"><span class=\"index\">1</span> Раздел</h2>"), "{page}");
        let css = String::from_utf8(paged.0.iter().find(|f| f.path == "style.css").unwrap().data.clone()).unwrap();
        assert!(!css.contains("display: none"), "стиль не прячет и не переставляет");
    }

    #[test]
    fn style_without_pages_warns() {
        let style = Style { page: None, ..Style::of(DocStyle::Gost7_32) };
        let mut warn = Vec::new();
        assert_eq!(geometry(&style, &mut warn).width, PageGeometry::A4.width);
        assert_eq!(warn, vec![RenderWarning { origin: Origin::Style(DocStyle::Gost7_32), file: None, kind: RenderWarningKind::Unsupported(Feature::Paging) }]);
    }
}
