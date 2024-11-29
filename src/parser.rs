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

#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    reason: String,
    line: String,
}
#[derive(Debug)]
pub enum ParseErrorKind {
    InvalidLineStart,
    ExpectationFailed,
    InvalidLine,
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

    fn parse_line_content<'line>(
        line: &'line str,
        kind: &Line,
    ) -> Result<&'line str, ParseError> {
        let content = match kind {
            Line::Command => line,
            Line::Index => {
                line.strip_prefix("index ").ok_or_else(|| ParseError {
                    kind: ParseErrorKind::ExpectationFailed,
                    reason: "expect line start with `index `".to_string(),
                    line: line.to_string(),
                })?
            }
            Line::OrignPath => {
                line.strip_prefix("--- ").ok_or_else(|| ParseError {
                    kind: ParseErrorKind::ExpectationFailed,
                    reason: "expect line start with `--- `".to_string(),
                    line: line.to_string(),
                })?
            }
            Line::NewPath => {
                line.strip_prefix("+++ ").ok_or_else(|| ParseError {
                    kind: ParseErrorKind::ExpectationFailed,
                    reason: "expect line start with `+++ `".to_string(),
                    line: line.to_string(),
                })?
            }
            Line::Hunk => {
                let end_offset =
                    line.find(" @@").ok_or_else(|| ParseError {
                        kind: ParseErrorKind::ExpectationFailed,
                        reason: "cannot find hunk end with ` @@`".to_string(),
                        line: line.to_string(),
                    })?;
                line.split_at(end_offset).0.strip_prefix("@@ ").ok_or_else(
                    || ParseError {
                        kind: ParseErrorKind::ExpectationFailed,
                        reason: "expect line start with `@@ `".to_string(),
                        line: line.to_string(),
                    },
                )?
            }
            Line::LineChange(_) => line.split_at(1).1,
            // this should be unreachable
            Line::Unknown => panic!("unknown line start"),
        };

        Ok(content)
    }

    pub fn parse_git_udiff(src: &str) -> Result<DiffComposition, ParseError> {
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
                Line::Unknown => Err(ParseError {
                    kind: ParseErrorKind::InvalidLineStart,
                    reason: "line starting with invalid token".to_string(),
                    line: line.to_string(),
                })?,
            };
            let content = Self::parse_line_content(line, &tag)?;
            match state {
                ParserState::Init => unreachable!(),
                ParserState::Command => {
                    if diff_cur.is_some() {
                        diffcom.diff.push(diff_cur.take().unwrap());
                    }
                    let (file_path_a, file_path_b) = content
                        .strip_prefix("diff --git ")
                        .and_then(|s|s.split_once(' '))
                        .ok_or_else(||{
                            ParseError{
                                kind:ParseErrorKind::ExpectationFailed,
                                reason:"lines not starting with `diff --git ` or cannot split command's arguments".to_string(
                                ),
                                line:line.to_string(),
                            }
                        })?;
                    let file_path_a = file_path_a
                        .strip_prefix("a/")
                        .ok_or_else(|| ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: "expect to path_a start with `a/`"
                                .to_string(),
                            line: line.to_string(),
                        })?;

                    let file_path_b = file_path_b
                        .strip_prefix("b/")
                        .ok_or_else(|| ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: "expect to path_a start with `b/`"
                                .to_string(),
                            line: line.to_string(),
                        })?;
                    if file_path_a != file_path_b {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: "file path a and b are different"
                                .to_string(),
                            line: line.to_string(),
                        })?;
                    }
                    let path = PathBuf::from_str(file_path_a).map_err(|e| {
                        ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: format!("cannot parse file_path, {:?}", e),
                            line: line.to_string(),
                        }
                    })?;

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
                            Err(ParseError {
                                kind: ParseErrorKind::InvalidLine,
                                reason: format!(
                                    "there is index in current diff {:?}",
                                    &diff_cur
                                ),
                                line: line.to_string(),
                            })?;
                        } else {
                            cur.index = Some(content.to_string())
                        }
                    }
                    None => {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: format!(
                                "there is no current diff {:?}",
                                &diff_cur
                            ),
                            line: line.to_string(),
                        })?;
                    }
                },
                ParserState::OriginPath => match &diff_cur {
                    Some(d) => {
                        let diff_path =
                            d.path.to_str().ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot convert diff path to str"
                                    .to_string(),
                                line: line.to_string(),
                            })?;

                        let origin_path = content
                            .strip_prefix("a/")
                            .ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "old file path not start with `a/`"
                                    .to_string(),
                                line: line.to_string(),
                            })?;
                        if diff_path != origin_path {
                            Err(ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: format!(
                                    "diff path and origin path is different, [diff: {}] [origin: {}]",
                                    diff_path, origin_path
                                ),
                                line: line.to_string(),
                            })?;
                        }
                    }
                    None => {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: format!(
                                "there is no current diff {:?}",
                                &diff_cur
                            ),
                            line: line.to_string(),
                        })?;
                    }
                },
                ParserState::NewPath => match &diff_cur {
                    Some(d) => {
                        let diff_path =
                            d.path.to_str().ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot convert diff path to str"
                                    .to_string(),
                                line: line.to_string(),
                            })?;

                        let new_path =
                            content.strip_prefix("b/").ok_or_else(|| {
                                ParseError {
                                    kind: ParseErrorKind::ExpectationFailed,
                                    reason: "old file path not start with `b/`"
                                        .to_string(),
                                    line: line.to_string(),
                                }
                            })?;
                        if diff_path != new_path {
                            Err(ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: format!(
                                    "diff path and new path is different, [diff: {}] [new: {}]",
                                    diff_path, new_path
                                ),
                                line: line.to_string(),
                            })?;
                        }
                    }
                    None => {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: format!(
                                "there is no current diff {:?}",
                                &diff_cur
                            ),
                            line: line.to_string(),
                        })?;
                    }
                },
                ParserState::Hunk => match &mut diff_cur {
                    Some(dc) => {
                        if let Some(hunk_before) = hunk_cur.take() {
                            dc.hunk.push(hunk_before)
                        }
                        let (old, new) =
                            content.split_once(' ').ok_or_else(|| {
                                ParseError {
                                    kind: ParseErrorKind::ExpectationFailed,
                                    reason: "there is no space in hunk line"
                                        .to_string(),
                                    line: line.to_string(),
                                }
                            })?;
                        let (old_line, old_len) =
                            old.split_once(',').ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot split hunk old range with `,`"
                                    .to_string(),
                                line: line.to_string(),
                            })?;
                        let (new_line, new_len) =
                            new.split_once(',').ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot split hunk new range with `,`"
                                    .to_string(),
                                line: line.to_string(),
                            })?;

                        let old_line = old_line
                            .strip_prefix('-')
                            .ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot strip `-` of old_line"
                                    .to_string(),
                                line: line.to_string(),
                            })?
                            .parse::<usize>()
                            .map_err(|e| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: format!(
                                    "cannot parse old_line to usize, {:?}",
                                    e
                                ),
                                line: line.to_string(),
                            })?;
                        let old_len =
                            old_len.parse::<usize>().map_err(|e| {
                                ParseError {
                                    kind: ParseErrorKind::ExpectationFailed,
                                    reason: format!(
                                        "cannot parse old_len to usize, {:?}",
                                        e
                                    ),
                                    line: line.to_string(),
                                }
                            })?;
                        let new_line = new_line
                            .strip_prefix('+')
                            .ok_or_else(|| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: "cannot strip `+` of new_line"
                                    .to_string(),
                                line: line.to_string(),
                            })?
                            .parse::<usize>()
                            .map_err(|e| ParseError {
                                kind: ParseErrorKind::ExpectationFailed,
                                reason: format!(
                                    "cannot parse new_line to usize, {:?}",
                                    e
                                ),
                                line: line.to_string(),
                            })?;
                        let new_len =
                            new_len.parse::<usize>().map_err(|e| {
                                ParseError {
                                    kind: ParseErrorKind::ExpectationFailed,
                                    reason: format!(
                                        "cannot parse new_len to usize, {:?}",
                                        e
                                    ),
                                    line: line.to_string(),
                                }
                            })?;

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
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectationFailed,
                            reason: format!(
                                "there is no current hunk. current diff {:?}",
                                &diff_cur
                            ),
                            line: line.to_string(),
                        })?;
                    }
                },
            }
        }

        if let Some(hunk) = hunk_cur {
            match &mut diff_cur {
                Some(c) => c.hunk.push(hunk),
                None => {
                    Err(ParseError {
                        kind: ParseErrorKind::ExpectationFailed,
                        reason: "there is no diff_cur to add hunk".to_string(),
                        line: "".to_string(),
                    })?;
                }
            }
        }
        if let Some(diff) = diff_cur {
            diffcom.diff.push(diff);
        } else {
            Err(ParseError {
                kind: ParseErrorKind::ExpectationFailed,
                reason: "there is no diff_cur at end".to_string(),
                line: "".to_string(),
            })?;
        }

        Ok(diffcom)
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
        let com = Parser::parse_git_udiff(short_test_data).unwrap();
        println!("{:#?}", com);
    }
}
