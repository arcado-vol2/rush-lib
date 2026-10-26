use std::fmt;

mod parse;
mod render;
mod style;
mod config;
mod resources;
mod content;
mod project;

pub use config::{Config, DocOut, DocStyle};
pub use project::{Project, ProjectWarning};
pub use render::{Feature, Origin, RenderWarning, RenderWarningKind};
pub use parse::common::RefKind;

use crate::content::Content;

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
    let (content, content_warns) = Content::resolve(&project.content, &project.resources);
    let mut warns: Vec<Warning> = content_warns.into_iter().map(Warning::from).collect();
    let (files, render_warns) = render::build(config, project, &content)?;
    warns.extend(render_warns.into_iter().map(Warning::from));
    Ok((files, warns))
}
