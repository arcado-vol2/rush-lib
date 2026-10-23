use std::{fmt, path::Path};

mod parse;
mod render;
mod config;
mod resources;
mod content;
mod project;

pub use config::{Config, DocOut, DocStyle};
pub use project::{Project, ProjectWarning};
pub use render::RenderWarning;
pub use parse::common::RefKind;

use crate::{content::Content, parse::rush::Rush, render::common::Backend};
use crate::render::html::{HtmlBackend, HtmlStyle, gost, gostlike::GostLike};
use crate::parse::block_fragmentation::Counters;

#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    Project(ProjectWarning),
    Render(RenderWarning),
}

impl From<ProjectWarning> for Warning {
    fn from(w: ProjectWarning) -> Self {
        Self::Project(w)
    }
}

impl From<RenderWarning> for Warning {
    fn from(w: RenderWarning) -> Self {
        Self::Render(w)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutFile {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn build(config: &Config, project: &Project) -> Result<(Vec<OutFile>, Vec<Warning>), fmt::Error> {
    let resources = &project.resources;
    let (content, content_warns) = Content::resolve(&project.content, resources);
    let mut warns: Vec<Warning> = content_warns.into_iter().map(Warning::from).collect();
    let mut files = Vec::new();
    match config.out {
        DocOut::Html if config.style == DocStyle::Gost => {
            let render_warns = build_gost(project, &content, &mut files);
            warns.extend(render_warns.into_iter().map(Warning::from));
        }
        DocOut::Html => {
            let style: &dyn HtmlStyle = match config.style {
                DocStyle::GostLike => &GostLike,
                _ => {
                    unimplemented!("not yet")
                }
            };
            let backend = HtmlBackend { style };
            files.push(OutFile { path: style.css_name().to_string(), data: style.css().into() });
            for (id, local, text) in resources.files.iter() {
                let rush = Rush::new(text, resources);
                let (html, render_warns) = backend.render(rush, resources, id, &content)?;
                warns.extend(render_warns.into_iter().map(Warning::from));
                files.push(OutFile { path: out_path(local.with_extension("html").as_path()), data: html.into_bytes() });
            }
        }
        _ => {
            unimplemented!("not yet")
        }
    }
    Ok((files, warns))
}

fn build_gost(project: &Project, content: &Content, files: &mut Vec<OutFile>) -> Vec<RenderWarning> {
    let resources = &project.resources;
    files.push(OutFile { path: gost::CSS_NAME.to_string(), data: gost::CSS.into() });
    for (name, data) in gost::metrics::FONT_FILES {
        files.push(OutFile { path: format!("{}/{name}", gost::FONTS_DIR), data: data.to_vec() });
    }

    let mut counters = Counters::default();
    let mut docs = Vec::with_capacity(content.items.len());
    for item in &content.items {
        let rush = Rush::new_from(resources.files.data(item.file), resources, counters);
        counters = rush.counters;
        docs.push(gost::Doc { file: item.file, name: &item.name, rush });
    }
    let (html, warns) = gost::render_project(docs, resources);
    files.push(OutFile { path: gost::OUT_NAME.to_string(), data: html.into_bytes() });
    warns
}

fn out_path(local: &Path) -> String {
    local.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}
