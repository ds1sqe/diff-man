use std::path::Path;

pub mod diff;

pub mod parser;

pub struct DiffManager {}
impl DiffManager {
    pub fn parse(
        format: &diff::DiffFormat,
        diff: &str,
    ) -> Result<diff::DiffComposition, parser::ParseError> {
        match format {
            diff::DiffFormat::GitUdiff => parser::Parser::parse_git_udiff(diff),
        }
    }

    pub fn apply(
        comp: &diff::DiffComposition,
        root: &Path,
    ) -> Result<(), diff::DiffError> {
        comp.apply(root)
    }

    pub fn revert(
        comp: &diff::DiffComposition,
        root: &Path,
    ) -> Result<(), diff::DiffError> {
        comp.revert(root)
    }
}
