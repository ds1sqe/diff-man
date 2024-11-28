use std::path::PathBuf;

const DIFF_SIGN_LINE_ADDED: &str = "+";
const DIFF_SIGN_LINE_DELETED: &str = "-";
const DIFF_SIGN_LINE_DEFAULT: &str = " ";
const DIFF_SIGN_HEADER_ORIGIN: &str = "---";
const DIFF_SIGN_HEADER_NEW: &str = "+++";
const DIFF_SIGN_HUNK: &str = "@@";

pub enum DiffFormat {
    GitUdiff,
}

pub struct DiffComposition {
    pub format: DiffFormat,
    pub diff: Vec<Diff>,
}
pub struct Diff {
    pub path: PathBuf,
    pub hunk: Vec<DiffHunk>,
    pub index: Option<String>, // TODO: type this
    pub command: Option<String>,
}
pub struct DiffHunk {
    pub old_line: u64,
    pub old_len: u64,
    pub new_line: u64,
    pub new_len: u64,
    pub change: Vec<LineChange>,
}
pub struct LineChange {
    kind: ChangeKind,
    content: String,
}

#[derive(Debug)]
pub enum ChangeKind {
    Default,
    Added,
    Deleted,
}

#[derive(Debug)]
pub enum LineStart {
    Command,
    Index,
    OrignPath,
    NewPath,
    Hunk,
    LineChange(ChangeKind),
    Unknown,
}

struct Parser {}
#[derive(Debug)]
enum ParserState {
    Init,
    Command,
    Index,
    OriginPath,
    NewPath,
    Hunk,
    LineChange,
}

impl Parser {
    fn parse_line_start(state: &ParserState, line: &str) -> LineStart {
        match state {
            ParserState::Init => {
                if line.starts_with("diff") {
                    LineStart::Command
                } else {
                    LineStart::Unknown
                }
            }
            ParserState::Command => {
                if line.starts_with("index") {
                    LineStart::Index
                } else if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    LineStart::OrignPath
                } else {
                    LineStart::Unknown
                }
            }
            ParserState::Index => {
                if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    LineStart::OrignPath
                } else {
                    LineStart::Unknown
                }
            }
            ParserState::OriginPath => {
                if line.starts_with(DIFF_SIGN_HEADER_NEW) {
                    LineStart::NewPath
                } else {
                    LineStart::Unknown
                }
            }
            ParserState::NewPath => {
                if line.starts_with(DIFF_SIGN_HUNK) {
                    LineStart::Hunk
                } else {
                    LineStart::Unknown
                }
            }
            ParserState::Hunk => match line.split_at(1) {
                (DIFF_SIGN_LINE_ADDED, _) => {
                    LineStart::LineChange(ChangeKind::Added)
                }
                (DIFF_SIGN_LINE_DEFAULT, _) => {
                    LineStart::LineChange(ChangeKind::Default)
                }
                (DIFF_SIGN_LINE_DELETED, _) => {
                    LineStart::LineChange(ChangeKind::Deleted)
                }
                _ => LineStart::Unknown,
            },
            ParserState::LineChange => {
                if line.starts_with("diff") {
                    LineStart::Command
                } else if line.starts_with("index") {
                    LineStart::Index
                } else if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    LineStart::OrignPath
                } else if line.starts_with(DIFF_SIGN_HUNK) {
                    LineStart::Hunk
                } else {
                    match line.split_at(1) {
                        (DIFF_SIGN_LINE_ADDED, _) => {
                            LineStart::LineChange(ChangeKind::Added)
                        }
                        (DIFF_SIGN_LINE_DEFAULT, _) => {
                            LineStart::LineChange(ChangeKind::Default)
                        }
                        (DIFF_SIGN_LINE_DELETED, _) => {
                            LineStart::LineChange(ChangeKind::Deleted)
                        }
                        _ => LineStart::Unknown,
                    }
                }
            }
        }
    }

    fn parse_git_udiff(src: &str) -> DiffComposition {
        let mut state = ParserState::Init;
        // State
        //  command     diff --git a/tests/vm.rs b/tests/vm.rs
        //  index       index 90d5af1..30044cb 100644
        //  old_path    --- a/tests/vm.rs
        //  new_path    +++ b/tests/vm.rs
        //  hunk        @@ -16,7 +16,9 @@
        //      linechange... |+|
        //      linechange... |-|
        //      linechange... | |
        //      *hunk        @@ ...

        for line in src.lines() {
            let linestart = Self::parse_line_start(&state, line);
            todo!()
        }
        todo!()
    }
}

pub struct DiffManager {}
impl DiffManager {
    pub fn parse() -> DiffComposition {
        todo!()
    }
}

mod test {
    use core::panic;

    use crate::{Parser, ParserState};

    const short_test_data: &str = r#"diff --git a/tests/vm.rs b/tests/vm.rs
index 90d5af1..30044cb 100644
--- a/tests/vm.rs
+++ b/tests/vm.rs
@@ -16,7 +16,9 @@ fn run_vm_test(tests: Tests<Option<Object>>) {
         let program = Parser::new(lexer).parse().unwrap();
 
         let mut comp = Compiler::create().unwrap();
-        comp.compile(program);
+        if let Err(e) = comp.compile(program) {
+            panic!("Compile error {:?}", e);
+        }
         let bytecode = comp.bytecode().unwrap();
 
         println!("Bytecode\n{}", bytecode.to_string());
@@ -25,7 +27,7 @@ fn run_vm_test(tests: Tests<Option<Object>>) {
 
         while vm.is_runable() {
             if let Err(err) = vm.run_single() {
-                eprintln!("Error {:?}", err);
+                panic!("VmError {:?}", err)
             }
         }
         println!("VM STACK:\n {}", vm.stack_to_string());
@@ -262,8 +264,7 @@ let no_return = fn() { };no_return() no_return() no_return() no_return()
 
     tests.add((
         "
-let fun = fn() { 10 + 20 };
-fun()
+let fun = fn() { 10 + 20 }; fun()
 ",
         Some(Object::Int(Int { value: 30 })),
     ));
"#;
    #[test]
    fn test_parse_linestart() {
        let mut state = ParserState::Init;
        for line in short_test_data.lines() {
            let tag = Parser::parse_line_start(&state, line);
            state = match &tag {
                crate::LineStart::Command => ParserState::Command,
                crate::LineStart::Index => ParserState::Index,
                crate::LineStart::OrignPath => ParserState::OriginPath,
                crate::LineStart::NewPath => ParserState::NewPath,
                crate::LineStart::Hunk => ParserState::Hunk,
                crate::LineStart::LineChange(change_kind) => {
                    ParserState::LineChange
                }
                crate::LineStart::Unknown => panic!("Unknown line start"),
            };

            println!("[S:{:?}] [T:{:?}] -- L>{}", &state, &tag, &line);
        }
    }
}
