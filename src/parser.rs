use {
    crate::diff::*,
    std::{path::PathBuf, str::FromStr},
};

pub struct Parser {}
#[derive(Debug)]
enum ParserState {
    Init,
    Command,
    Index,
    OriginPath,
    NewPath,
    Hunk,
    LineChange(Change),
}

impl Parser {
    fn parse_line_kind(state: &ParserState, line: &str) -> Line {
        match state {
            ParserState::Init => {
                if line.starts_with("diff") {
                    Line::Command
                } else {
                    Line::Unknown
                }
            }
            ParserState::Command => {
                if line.starts_with("index") {
                    Line::Index
                } else if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    Line::OrignPath
                } else {
                    Line::Unknown
                }
            }
            ParserState::Index => {
                if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    Line::OrignPath
                } else {
                    Line::Unknown
                }
            }
            ParserState::OriginPath => {
                if line.starts_with(DIFF_SIGN_HEADER_NEW) {
                    Line::NewPath
                } else {
                    Line::Unknown
                }
            }
            ParserState::NewPath => {
                if line.starts_with(DIFF_SIGN_HUNK) {
                    Line::Hunk
                } else {
                    Line::Unknown
                }
            }
            ParserState::Hunk => match line.split_at(1) {
                (DIFF_SIGN_LINE_ADDED, _) => Line::LineChange(Change::Added),
                (DIFF_SIGN_LINE_DEFAULT, _) => {
                    Line::LineChange(Change::Default)
                }
                (DIFF_SIGN_LINE_DELETED, _) => {
                    Line::LineChange(Change::Deleted)
                }
                _ => Line::Unknown,
            },
            ParserState::LineChange(_) => {
                if line.starts_with("diff") {
                    Line::Command
                } else if line.starts_with("index") {
                    Line::Index
                } else if line.starts_with(DIFF_SIGN_HEADER_ORIGIN) {
                    Line::OrignPath
                } else if line.starts_with(DIFF_SIGN_HUNK) {
                    Line::Hunk
                } else {
                    match line.split_at(1) {
                        (DIFF_SIGN_LINE_ADDED, _) => {
                            Line::LineChange(Change::Added)
                        }
                        (DIFF_SIGN_LINE_DEFAULT, _) => {
                            Line::LineChange(Change::Default)
                        }
                        (DIFF_SIGN_LINE_DELETED, _) => {
                            Line::LineChange(Change::Deleted)
                        }
                        _ => Line::Unknown,
                    }
                }
            }
        }
    }

    fn parse_line_content<'line>(line: &'line str, kind: &Line) -> &'line str {
        match kind {
            Line::Command => line,
            Line::Index => line
                .strip_prefix("index ")
                .expect("expect line start with `index `"),
            Line::OrignPath => line
                .strip_prefix("--- ")
                .expect("expect line start with `--- `"),
            Line::NewPath => line
                .strip_prefix("+++ ")
                .expect("expect line start with `+++ `"),
            Line::Hunk => {
                let end_offset =
                    line.find(" @@").expect("cannot find hunk end with ` @@`");
                line.split_at(end_offset)
                    .0
                    .strip_prefix("@@ ")
                    .expect("expect line start with `@@ `")
            }
            Line::LineChange(_) => line.split_at(1).1,
            Line::Unknown => panic!("unknown line start"),
        }
    }

    pub fn parse_git_udiff(src: &str) -> DiffComposition {
        let mut state = ParserState::Init;
        // State
        //  command     diff --git a/tests/vm.rs b/tests/vm.rs
        //  index       index 90d5af1..30044cb 100644
        //  old_path    --- a/tests/vm.rs
        //  new_path    +++ b/tests/vm.rs
        //  hunk        @@ -16,7 +16,9 @@ scope..
        //      linechange... |+|
        //      linechange... |-|
        //      linechange... | |
        //      *hunk        @@ ...
        let mut diffcom = DiffComposition {
            format: DiffFormat::GitUdiff,
            diff: Vec::new(),
        };

        let mut diff_cur: Option<Diff> = None;
        let mut hunk_cur: Option<DiffHunk> = None;

        for line in src.lines() {
            let tag = Self::parse_line_kind(&state, line);
            state = match &tag {
                Line::Command => ParserState::Command,
                Line::Index => ParserState::Index,
                Line::OrignPath => ParserState::OriginPath,
                Line::NewPath => ParserState::NewPath,
                Line::Hunk => ParserState::Hunk,
                Line::LineChange(change_kind) => {
                    ParserState::LineChange(*change_kind)
                }
                Line::Unknown => panic!("Unknown line start"),
            };
            let content = Self::parse_line_content(line, &tag);
            match state {
                ParserState::Init => unreachable!(),
                ParserState::Command => {
                    if diff_cur.is_some() {
                        diffcom.diff.push(diff_cur.take().unwrap());
                    }
                    let (file_path_a, file_path_b) = content
                        .strip_prefix("diff --git ")
                        .expect("expect to command start with `diff --git `")
                        .split_once(' ')
                        .expect("cannot split command's arguments");
                    let file_path_a = file_path_a
                        .strip_prefix("a/")
                        .expect("expect to path_a start with `a/`");
                    let file_path_b = file_path_b
                        .strip_prefix("b/")
                        .expect("expect to path_a start with `b/`");
                    assert_eq!(file_path_a, file_path_b);
                    let path = PathBuf::from_str(file_path_a)
                        .expect("cannot parse file_path");

                    diff_cur = Some(Diff {
                        path,
                        hunk: Vec::new(),
                        command: Some(content.to_string()),
                        index: None,
                    });
                }
                ParserState::Index => match &mut diff_cur {
                    Some(cur) => {
                        if cur.index.is_some() {
                            panic!(
                                "there is index in current diff {:?}",
                                &diff_cur
                            )
                        } else {
                            cur.index = Some(content.to_string())
                        }
                    }
                    None => {
                        panic!("there is no current diff {:?}", &diff_cur)
                    }
                },
                ParserState::OriginPath => match &diff_cur {
                    Some(d) => {
                        assert_eq!(
                            d.path
                                .to_str()
                                .expect("cannot convert diff path to str"),
                            content
                                .strip_prefix("a/")
                                .expect("old file path not start with `a/`")
                        )
                    }
                    None => {
                        panic!("there is no current diff {:?}", &diff_cur)
                    }
                },
                ParserState::NewPath => match &diff_cur {
                    Some(d) => {
                        assert_eq!(
                            d.path
                                .to_str()
                                .expect("cannot convert diff path to str"),
                            content
                                .strip_prefix("b/")
                                .expect("old file path not start with `b/`")
                        )
                    }
                    None => {
                        panic!("there is no current diff {:?}", &diff_cur)
                    }
                },
                ParserState::Hunk => match &mut diff_cur {
                    Some(dc) => {
                        if let Some(hunk_before) = hunk_cur.take() {
                            dc.hunk.push(hunk_before)
                        }
                        let (old, new) = content
                            .split_once(' ')
                            .expect("there is no space in hunk line");
                        let (old_line, old_len) = old
                            .split_once(',')
                            .expect("cannot split hunk old range with `,`");
                        let (new_line, new_len) = new
                            .split_once(',')
                            .expect("cannot split hunk new range with `,`");
                        let old_line = old_line
                            .strip_prefix('-')
                            .expect("cannot strip `-` of old_line")
                            .parse::<usize>()
                            .expect("cannot parse old_line");
                        let old_len = old_len
                            .parse::<usize>()
                            .expect("cannot parse old_len");
                        let new_line = new_line
                            .strip_prefix('+')
                            .expect("cannot strip `+` of new_line")
                            .parse::<usize>()
                            .expect("cannot parse new_line");
                        let new_len = new_len
                            .parse::<usize>()
                            .expect("cannot parse new_len");

                        hunk_cur = Some(DiffHunk {
                            old_line,
                            old_len,
                            new_line,
                            new_len,
                            change: Vec::new(),
                        });
                    }
                    None => {
                        panic!("there is no current diff {:?}", &diff_cur)
                    }
                },
                ParserState::LineChange(kind) => match &mut hunk_cur {
                    Some(h) => {
                        let change = LineChange {
                            kind,
                            content: content.to_string(),
                        };
                        h.change.push(change)
                    }
                    None => {
                        panic!(
                            "there is no current hunk. current diff {:?}",
                            &diff_cur
                        )
                    }
                },
            }
        }

        if let Some(hunk) = hunk_cur {
            match &mut diff_cur {
                Some(c) => c.hunk.push(hunk),
                None => panic!("there is no diff_cur to add hunk"),
            }
        }
        if let Some(diff) = diff_cur {
            diffcom.diff.push(diff);
        } else {
            panic!("there is no diff_cur at end")
        }

        diffcom
    }
}

#[cfg(test)]
mod test {
    use core::panic;

    use crate::{
        diff::*,
        parser::{Parser, ParserState},
    };

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
            let tag = Parser::parse_line_kind(&state, line);
            state = match &tag {
                Line::Command => ParserState::Command,
                Line::Index => ParserState::Index,
                Line::OrignPath => ParserState::OriginPath,
                Line::NewPath => ParserState::NewPath,
                Line::Hunk => ParserState::Hunk,
                Line::LineChange(change_kind) => {
                    ParserState::LineChange(*change_kind)
                }
                Line::Unknown => panic!("Unknown line start"),
            };

            println!("[S:{:?}] [T:{:?}] -- L>{}", &state, &tag, &line);
        }
    }

    #[test]
    fn test_parse_udiff() {
        let com = Parser::parse_git_udiff(short_test_data);
        println!("{:#?}", com);
    }
}
