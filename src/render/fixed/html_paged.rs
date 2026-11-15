use super::{Item, Layout, LineBox};
use crate::config::DocOut;
use crate::render::warning::Feature;
use crate::render::{css_fonts, escape, Fixed};
use crate::style::Style;
use crate::OutFile;
use std::fmt::Write;

const BASE_CSS: &str = include_str!("../../templates/html_paged/base.css");
pub(crate) const CSS_NAME: &str = "style.css";
pub(crate) const OUT_NAME: &str = "index.html";

pub(crate) struct HtmlPaged;

fn line(out: &mut String, y: f32, l: &LineBox) {
    let _ = write!(out, "<div class=\"l\" style=\"top:{y:.2}pt;left:{:.2}pt;font-size:{}pt;line-height:{}pt;height:{}pt", l.x, l.size, l.lh, l.lh);
    if l.word_spacing > 0.0 {
        let _ = write!(out, ";word-spacing:{:.3}pt", l.word_spacing);
    }
    out.push_str("\">");
    for run in &l.runs {
        let mut close: Vec<&str> = Vec::new();
        if let Some(href) = &run.link {
            out.push_str("<a href=\"");
            let _ = escape(out, href);
            out.push_str("\">");
            close.push("</a>");
        }
        let tags = [
            (run.font.bold, "<b>", "</b>"),
            (run.font.italic, "<i>", "</i>"),
            (run.font.mono && !run.error, "<span class=\"mono\">", "</span>"),
            (run.strike, "<s>", "</s>"),
            (run.under, "<u>", "</u>"),
            (run.sup, "<sup class=\"fr\">", "</sup>"),
            (run.error, "<span class=\"error\">", "</span>"),
        ];
        for (on, open, c) in tags {
            if on {
                out.push_str(open);
                close.push(c);
            }
        }
        if let Some(col) = &run.color {
            let _ = write!(out, "<span style=\"color:{}\">", col.to_hex_tag());
            close.push("</span>");
        }
        if let Some(bg) = &run.bg {
            let _ = write!(out, "<span style=\"background:{}\">", bg.to_hex_tag());
            close.push("</span>");
        }
        let _ = escape(out, &run.text);
        for c in close.iter().rev() {
            out.push_str(c);
        }
    }
    out.push_str("</div>");
}

fn item(out: &mut String, item: &Item) {
    match item {
        Item::Line { y, line: l } => line(out, *y, l),
        Item::Rule { x, y, w, h } => {
            let _ = write!(out, "<div class=\"rule\" style=\"left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt;height:{h:.2}pt\"></div>");
        }
        Item::Fill { x, y, w, h, color } => {
            let _ = write!(out, "<div class=\"fill\" style=\"left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt;height:{h:.2}pt;background:{}\"></div>", color.to_hex_tag());
        }
        Item::Leader { x, y, w } => {
            let _ = write!(out, "<div class=\"leader\" style=\"left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt\"></div>");
        }
        Item::Image { x, y, w, h, src } => {
            out.push_str("<img alt=\"\" src=\"");
            let _ = escape(out, src);
            let _ = write!(out, "\" style=\"left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt;height:{h:.2}pt\">");
        }
        Item::Math { x, y, w, h, size, mathml } => {
            let _ = write!(out, "<div class=\"math\" style=\"left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt;height:{h:.2}pt;font-size:{size}pt\">{mathml}</div>");
        }
        Item::Anchor { y, id } => {
            let _ = write!(out, "<a class=\"anchor\" id=\"{id}\" style=\"top:{y:.2}pt\"></a>");
        }
    }
}

fn css(style: &Style, layout: &Layout) -> String {
    let g = &layout.geometry;
    let mut out = css_fonts(style, "fonts");
    let _ = write!(out, "@page {{ size: {}pt {}pt; margin: 0; }}\n", g.width, g.height);
    let _ = write!(out, ":root {{ --page-w: {}pt; --page-h: {}pt; --m-top: {}pt; --m-left: {}pt; --text-w: {}pt; --text-h: {}pt; \
        --body: \"{}\", {}; --mono: \"{}\", {}; --paper: {}; --ink: {}; }}\n",
        g.width, g.height, g.top, g.left, g.text_w(), g.text_h(),
        style.fonts.body.name, style.fonts.body.fallback, style.fonts.mono.name, style.fonts.mono.fallback,
        style.chrome.paper.to_hex_tag(), style.chrome.ink.to_hex_tag());
    out.push_str(BASE_CSS);
    out
}

impl Fixed for HtmlPaged {
    fn out(&self) -> DocOut {
        DocOut::Html
    }

    fn supports(&self, _feature: Feature) -> bool {
        true
    }

    fn paint(&self, layout: &Layout, style: &Style) -> Vec<OutFile> {
        let mut html = String::from("<!DOCTYPE html><html lang=\"ru\"><head><meta charset=\"UTF-8\">");
        html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>");
        let _ = escape(&mut html, &layout.title);
        let _ = write!(html, "</title><link rel=\"stylesheet\" href=\"{CSS_NAME}\"></head><body>");
        for page in &layout.pages {
            html.push_str("<section class=\"page\"><div class=\"content\">");
            for i in &page.items {
                item(&mut html, i);
            }
            html.push_str("</div></section>");
        }
        html.push_str("</body></html>");
        vec![
            OutFile { path: CSS_NAME.to_string(), data: css(style, layout).into_bytes() },
            OutFile { path: OUT_NAME.to_string(), data: html.into_bytes() },
        ]
    }
}
