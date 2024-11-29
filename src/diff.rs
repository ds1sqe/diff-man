use std::{
    fs,
    path::{Path, PathBuf},
};

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
    pub fn apply(&self, root: &Path) {
        for diff in &self.diff {
            let target_path = root.join(&diff.path);
            let original =
                fs::read_to_string(&target_path).expect("cannot read");
            let after = diff.apply(&original);
            fs::write(target_path, after).expect("failed to apply");
        }
    }
    pub fn revert(&self, root: &Path) {
        for diff in &self.diff {
            let target_path = root.join(&diff.path);
            let applied =
                fs::read_to_string(&target_path).expect("cannot read");
            let after = diff.revert(&applied);
            fs::write(target_path, after).expect("failed to revert");
        }
    }
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

        while oidx < lines.len() {
            buffer
                .push_str(lines.get(oidx).expect("there is no line in lines"));
            buffer.push('\n');
            oidx += 1;
        }

        buffer
    }

    pub fn revert(&self, applied: &str) -> String {
        let mut buffer = String::new();

        let mut aidx: usize = 0;
        let lines: Vec<&str> = applied.lines().collect();

        for hunk in &self.hunk {
            while aidx < (hunk.new_line - 1) {
                buffer.push_str(
                    lines.get(aidx).expect("there is no line in lines"),
                );
                buffer.push('\n');
                aidx += 1;
            }
            for change in &hunk.change {
                match change.kind {
                    Change::Default => {
                        let content =
                            lines.get(aidx).expect("there is no line in lines");
                        assert_eq!(&change.content, content);
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

        buffer
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

    #[test]
    fn test_diff_apply_middle() {
        let original = fs::read_to_string("test_data/middle.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/middle.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file);
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original);

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

        assert_eq!(applied.as_str(), expected.as_str());
        let before = diff.revert(&applied);
        assert_eq!(before.as_str(), original.as_str())
    }

    #[test]
    fn test_diff_apply_revert_middle() {
        let original = fs::read_to_string("test_data/middle.before").unwrap();
        println!("<<original start>>");
        print!("{}", original);
        println!("<<original end>>");

        let diff_file = fs::read_to_string("test_data/middle.diffs").unwrap();

        let com = Parser::parse_git_udiff(&diff_file);
        println!("{:#?}", com);
        let diff = com.diff.first().unwrap();
        let applied = diff.apply(&original);

        println!("<<applied start>>");
        print!("{}", applied);
        println!("<<applied end>>");

        let expected = fs::read_to_string("test_data/middle.after").unwrap();
        println!("<<expected start>>");
        print!("{}", expected);
        println!("<<expected end>>");

        assert_eq!(applied.as_str(), expected.as_str());
        let before = diff.revert(&applied);
        assert_eq!(before.as_str(), original.as_str())
    }

    #[test]
    fn test_comp_simple_apply() {
        let diff_file =
            fs::read_to_string("test_data/composition/simple_app.diffs")
                .unwrap();
        let com = Parser::parse_git_udiff(&diff_file);
        fs::copy(
            "test_data/simple.before",
            "test_data/composition/simple_app",
        )
        .expect("failed to copy");
        let comp_root = PathBuf::from_str("test_data/composition").unwrap();
        com.apply(&comp_root);
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
        let com = Parser::parse_git_udiff(&diff_file);
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
        let com = Parser::parse_git_udiff(&diff_file);
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
        let com = Parser::parse_git_udiff(&diff_file);
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
