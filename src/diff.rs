use std::path::{Path, PathBuf};

pub const DIFF_SIGN_LINE_ADDED: &str = "+";
pub const DIFF_SIGN_LINE_DELETED: &str = "-";
pub const DIFF_SIGN_LINE_DEFAULT: &str = " ";
pub const DIFF_SIGN_HEADER_ORIGIN: &str = "---";
pub const DIFF_SIGN_HEADER_NEW: &str = "+++";
pub const DIFF_SIGN_HUNK: &str = "@@";

#[derive(Debug)]
pub enum DiffFormat {
    GitUdiff,
}

#[derive(Debug)]
pub struct DiffComposition {
    pub format: DiffFormat,
    pub diff: Vec<Diff>,
}
#[derive(Debug)]
pub struct Diff {
    pub command: Option<String>,
    pub index: Option<String>, // TODO: type this
    pub path: PathBuf,
    pub hunk: Vec<DiffHunk>,
}
#[derive(Debug)]
pub struct DiffHunk {
    pub old_line: usize,
    pub old_len: usize,
    pub new_line: usize,
    pub new_len: usize,
    pub change: Vec<LineChange>,
}
#[derive(Debug)]
pub struct LineChange {
    pub kind: Change,
    pub content: String,
}

#[derive(Debug, Copy, Clone)]
pub enum Change {
    Default,
    Added,
    Deleted,
}

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

impl DiffComposition {
    pub fn apply(&self, root: &Path) {}
}

impl Diff {
    pub fn apply(&self, original: &str) -> String {
        let mut buffer = String::new();

        // index of original line
        let mut oidx: usize = 0;
        let lines: Vec<&str> = original.lines().collect();

        for hunk in &self.hunk {
            while oidx < (hunk.old_line - 1) {
                buffer.push_str(
                    lines.get(oidx).expect("there is no line in lines"),
                );
                buffer.push('\n');
                oidx += 1;
            }
            for change in &hunk.change {
                match change.kind {
                    Change::Default => {
                        buffer.push_str(
                            lines.get(oidx).expect("there is no line in lines"),
                        );
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

        buffer
    }
}

#[cfg(test)]
mod test {
    use {core::panic, std::fs};

    use crate::{diff::*, parser::Parser};

    #[test]
    fn test_diff_apply() {
        let original = fs::read_to_string("test_data/simple.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/simple.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file);
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original);

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/simple.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str())
    }
}
