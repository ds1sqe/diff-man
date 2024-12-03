use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const DIFF_SIGN_LINE_ADDED: &str = "+";
pub const DIFF_SIGN_LINE_DELETED: &str = "-";
pub const DIFF_SIGN_LINE_DEFAULT: &str = " ";
pub const DIFF_SIGN_HEADER_ORIGIN: &str = "---";
pub const DIFF_SIGN_HEADER_NEW: &str = "+++";
pub const DIFF_SIGN_HUNK: &str = "@@";

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub enum DiffFormat {
    GitUdiff,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub enum DiffFormat {
    GitUdiff,
}

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub struct DiffComposition {
    pub format: DiffFormat,
    pub diff: Vec<Diff>,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub struct DiffComposition {
    pub format: DiffFormat,
    pub diff: Vec<Diff>,
}
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub struct Diff {
    pub command: Option<String>,
    pub index: Option<String>, // TODO: type this
    pub path: PathBuf,
    pub hunk: Vec<DiffHunk>,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub struct Diff {
    pub command: Option<String>,
    pub index: Option<String>, // TODO: type this
    pub path: PathBuf,
    pub hunk: Vec<DiffHunk>,
}
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub struct DiffHunk {
    pub old_line: usize,
    pub old_len: usize,
    pub new_line: usize,
    pub new_len: usize,
    pub change: Vec<LineChange>,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub struct DiffHunk {
    pub old_line: usize,
    pub old_len: usize,
    pub new_line: usize,
    pub new_len: usize,
    pub change: Vec<LineChange>,
}
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub struct LineChange {
    pub kind: Change,
    pub content: String,
}

#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub struct LineChange {
    pub kind: Change,
    pub content: String,
}
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum Change {
    Default,
    Added,
    Deleted,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug, Copy, Clone)]
pub enum Change {
    Default,
    Added,
    Deleted,
}

#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize, Debug)]
pub enum Line {
    Command,
    Index,
    OrignPath,
    NewPath,
    Hunk,
    LineChange(Change),
    Unknown,
}
#[cfg(not(feature = "serde"))]
#[derive(Debug)]
pub enum Line {
    Command,
    Index,
    OrignPath,
    NewPath,
    Hunk,
    LineChange(Change),
    Unknown,
}

#[derive(Debug)]
pub struct DiffError {
    kind: DiffErrorKind,
    reason: String,
}
#[derive(Debug)]
pub enum DiffErrorKind {
    IOError(io::Error),
    InvalidIndex(usize),
    UnmatchedContent(String, String),
}

impl From<io::Error> for DiffError {
    fn from(e: io::Error) -> Self {
        DiffError {
            kind: DiffErrorKind::IOError(e),
            reason: "cannot read or write".to_string(),
        }
    }
}

impl DiffComposition {
    pub fn apply(&self, root: &Path) -> Result<(), DiffError> {
        for diff in &self.diff {
            let target_path = root.join(&diff.path);
            let original = fs::read_to_string(&target_path)?;
            let after = diff.apply(&original)?;
            fs::write(target_path, after)?;
        }
        Ok(())
    }
    pub fn revert(&self, root: &Path) -> Result<(), DiffError> {
        for diff in &self.diff {
            let target_path = root.join(&diff.path);
            let applied = fs::read_to_string(&target_path)?;
            let after = diff.revert(&applied)?;
            fs::write(target_path, after)?;
        }
        Ok(())
    }
}

impl Diff {
    pub fn apply(&self, original: &str) -> Result<String, DiffError> {
        let mut buffer = String::new();

        // index of original line
        let mut oidx: usize = 0;
        let lines: Vec<&str> = original.lines().collect();

        for hunk in &self.hunk {
            while oidx < (hunk.old_line - 1) {
                buffer.push_str(lines.get(oidx).ok_or_else(|| DiffError {
                    kind: DiffErrorKind::InvalidIndex(oidx),
                    reason: format!("cannot get line at {oidx}"),
                })?);
                buffer.push('\n');
                oidx += 1;
            }
            for change in &hunk.change {
                match change.kind {
                    Change::Default => {
                        buffer.push_str(lines.get(oidx).ok_or_else(|| {
                            DiffError {
                                kind: DiffErrorKind::InvalidIndex(oidx),
                                reason: format!("cannot get line at {oidx}"),
                            }
                        })?);
                        buffer.push('\n');
                        oidx += 1;
                    }
                    Change::Deleted => {
                        oidx += 1;
                        continue;
                    }
                    Change::Added => {
                        buffer.push_str(&change.content);
                        buffer.push('\n');
                    }
                }
            }
        }

        while oidx < lines.len() {
            buffer
                .push_str(lines.get(oidx).expect("there is no line in lines"));
            buffer.push('\n');
            oidx += 1;
        }

        Ok(buffer)
    }

    pub fn revert(&self, applied: &str) -> Result<String, DiffError> {
        let mut buffer = String::new();

        let mut aidx: usize = 0;
        let lines: Vec<&str> = applied.lines().collect();

        for hunk in &self.hunk {
            while aidx < (hunk.new_line - 1) {
                buffer.push_str(lines.get(aidx).ok_or_else(|| DiffError {
                    kind: DiffErrorKind::InvalidIndex(aidx),
                    reason: format!("cannot get line at {aidx}"),
                })?);
                buffer.push('\n');
                aidx += 1;
            }
            for change in &hunk.change {
                match change.kind {
                    Change::Default => {
                        let content =
                            lines.get(aidx).ok_or_else(|| DiffError {
                                kind: DiffErrorKind::InvalidIndex(aidx),
                                reason: format!("cannot get line at {aidx}"),
                            })?;
                        if change.content != *content {
                            Err(DiffError {
                                kind: DiffErrorKind::UnmatchedContent(
                                    change.content.to_string(),
                                    content.to_string(),
                                ),
                                reason: format!(
                                    "(change) : {:?}, line(content) : {}, @line_idx : {aidx}, Buffer:{}, hunk current {:#?}",
                                    change, content, buffer, hunk
                                ),
                            })?;
                        }
                        buffer.push_str(content);
                        buffer.push('\n');
                        aidx += 1;
                    }
                    Change::Deleted => {
                        buffer.push_str(&change.content);
                        buffer.push('\n');
                    }
                    Change::Added => {
                        aidx += 1;
                        continue;
                    }
                }
            }
        }

        while aidx < lines.len() {
            buffer
                .push_str(lines.get(aidx).expect("there is no line in lines"));
            buffer.push('\n');
            aidx += 1;
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use {core::panic, std::fs};

    use crate::{diff::*, parser::Parser};

    #[test]
    fn test_diff_apply_simple() {
        let original = fs::read_to_string("test_data/simple.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/simple.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original).unwrap();

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/simple.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str())
    }

    #[test]
    fn test_diff_apply_middle() {
        let original = fs::read_to_string("test_data/middle.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/middle.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original).unwrap();

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/middle.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str())
    }

    #[test]
    fn test_diff_apply_revert_simple() {
        let original = fs::read_to_string("test_data/simple.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/simple.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original).unwrap();

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/simple.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str());
        let before = diff.revert(&applied).unwrap();
        assert_eq!(before.as_str(), original.as_str())
    }

    #[test]
    fn test_diff_apply_revert_middle() {
        let original = fs::read_to_string("test_data/middle.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/middle.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original).unwrap();

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/middle.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str());
        let before = diff.revert(&applied).unwrap();
        assert_eq!(before.as_str(), original.as_str())
    }

    #[test]
    fn test_comp_simple_apply() {
        let diff_file =
            fs::read_to_string("test_data/composition/simple_app.diffs")
                .unwrap();
        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        fs::copy(
            "test_data/simple.before",
            "test_data/composition/simple_app",
        )
        .expect("failed to copy");
        let comp_root = PathBuf::from_str("test_data/composition").unwrap();
        com.apply(&comp_root).unwrap();
        let applied =
            fs::read_to_string("test_data/composition/simple_app").unwrap();
        let expected = fs::read_to_string("test_data/simple.after").unwrap();
        assert_eq!(applied.as_str(), expected.as_str());
        fs::remove_file("test_data/composition/simple_app")
            .expect("failed to remove file");
    }

    #[test]
    fn test_comp_simple_revert() {
        let diff_file =
            fs::read_to_string("test_data/composition/simple_rev.diffs")
                .unwrap();
        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        fs::copy("test_data/simple.after", "test_data/composition/simple_rev")
            .expect("failed to copy");
        let comp_root = PathBuf::from_str("test_data/composition").unwrap();
        com.revert(&comp_root);
        let reverted =
            fs::read_to_string("test_data/composition/simple_rev").unwrap();
        let expected = fs::read_to_string("test_data/simple.before").unwrap();
        assert_eq!(reverted.as_str(), expected.as_str());
        fs::remove_file("test_data/composition/simple_rev")
            .expect("failed to remove file");
    }

    #[test]
    fn test_comp_middle_apply() {
        let diff_file =
            fs::read_to_string("test_data/composition/middle_app.diffs")
                .unwrap();
        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        fs::copy(
            "test_data/middle.before",
            "test_data/composition/middle_app",
        )
        .expect("failed to copy");
        let comp_root = PathBuf::from_str("test_data/composition").unwrap();
        com.apply(&comp_root);
        let applied =
            fs::read_to_string("test_data/composition/middle_app").unwrap();
        let expected = fs::read_to_string("test_data/middle.after").unwrap();
        assert_eq!(applied.as_str(), expected.as_str());
        fs::remove_file("test_data/composition/middle_app")
            .expect("failed to remove file");
    }

    #[test]
    fn test_comp_middle_revert() {
        let diff_file =
            fs::read_to_string("test_data/composition/middle_rev.diffs")
                .unwrap();
        let com = Parser::parse_git_udiff(&diff_file).unwrap();
        fs::copy("test_data/middle.after", "test_data/composition/middle_rev")
            .expect("failed to copy");
        let comp_root = PathBuf::from_str("test_data/composition").unwrap();
        com.revert(&comp_root);
        let reverted =
            fs::read_to_string("test_data/composition/middle_rev").unwrap();
        let expected = fs::read_to_string("test_data/middle.before").unwrap();
        assert_eq!(reverted.as_str(), expected.as_str());
        fs::remove_file("test_data/composition/middle_rev")
            .expect("failed to remove file");
    }
}
