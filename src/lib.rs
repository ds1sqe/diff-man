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
pub enum ChangeKind {
    Default,
    Added,
    Deleted,
}

struct Parser {}

impl Parser {
    fn parse_git_udiff(src: &str) -> Diff {
        todo!()
    }
}

pub struct DiffManager {}
impl DiffManager {
    pub fn parse() -> DiffComposition {
        todo!()
    }
}

#[test]
fn parse_udiff() {
    let tgt = r#"diff --git a/tests/vm.rs b/tests/vm.rs
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
}
