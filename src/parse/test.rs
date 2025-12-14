use super::*;
use winnow::LocatingSlice;

type SpannedInput<'a> = (&'a str, Span);

fn literal(literal: SpannedInput) -> Spanned<Word> {
    Spanned {
        inner: Word::Literal(literal.0.to_string()),
        span: literal.1,
    }
}
fn path_literal(path_literal: SpannedInput) -> Spanned<Word> {
    Spanned {
        inner: Word::PathLiteral(path_literal.0.to_string()),
        span: path_literal.1,
    }
}
fn env_var(var: SpannedInput) -> Spanned<Word> {
    Spanned {
        inner: Word::EnvVar(var.0.to_string()),
        span: var.1,
    }
}
fn shell_var(var: SpannedInput) -> Spanned<Word> {
    Spanned {
        inner: Word::ShellVar(var.0.to_string()),
        span: var.1,
    }
}
fn special_var(var: SpecialVar, span: Span) -> Spanned<Word> {
    Spanned {
        inner: Word::SpecialVar(var),
        span,
    }
}
fn shell(command: SpannedInput, args: &[SpannedInput], comment: Option<&str>) -> ShellCommand {
    ShellCommand {
        commands: vec![(
            Command {
                name: literal(command),
                args: args.iter().cloned().map(literal).collect(),
            },
            None,
        )],
        comment: comment.map(str::to_string),
    }
}
fn shell_parse(
    input: &str,
) -> Result<ShellCommand, winnow::error::ParseError<LocatingSlice<&str>, ParseError>> {
    super::shell_command.parse(LocatingSlice::new(input))
}
fn word_parse(
    input: &str,
) -> Result<Spanned<Word>, winnow::error::ParseError<LocatingSlice<&str>, ParseError>> {
    super::word.parse(LocatingSlice::new(input))
}
fn word_peek(
    input: &str,
) -> Result<(LocatingSlice<&str>, Spanned<Word>), winnow::error::ErrMode<ParseError>> {
    super::word.parse_peek(LocatingSlice::new(input))
}

#[test]
fn double_quoted_string_test() {
    macro_rules! word_parse {
        ($input: expr) => {
            word_parse(concat!("\"", $input, "\""))
        };
    };
    assert_eq!(word_parse!("hello"), Ok(literal(("hello", 0..7))));
    assert_eq!(word_parse!("hello world"), Ok(literal("hello world")));
    assert_eq!(word_parse!(""), Ok(literal("")));
    assert_eq!(word_parse!(r"hello\nworld"), Ok(literal("hello\nworld")));
    assert_eq!(word_parse!(r"hello\tworld"), Ok(literal("hello\tworld")));
    assert_eq!(word_parse!(r"hello\rworld"), Ok(literal("hello\rworld")));
    assert_eq!(word_parse!(r"hello\0world"), Ok(literal("hello\0world")));
    assert_eq!(word_parse!(r"hello\\world"), Ok(literal("hello\\world")));
    assert_eq!(word_parse!("say \\\"hi\\\""), Ok(literal("say \"hi\"")));
    assert_eq!(word_parse!(r"it\'s"), Ok(literal("it\'s")));
    assert_eq!(word_parse!(r"\u{41}"), Ok(literal("\u{41}")));
    assert_eq!(word_parse!(r"\u{3042}"), Ok(literal("\u{3042}")));
    assert_eq!(word_parse!(r"\u{1F600}"), Ok(literal("\u{1F600}")));
    assert_eq!(
        word_parse!(r"line1\nline2\nline3"),
        Ok(literal("line1\nline2\nline3"))
    );

    assert!(word_parse!(r"\x").is_err());
    assert!(word_parse!(r"\u{GGGG}").is_err());
    assert!(word_parse!(r"\u{110000}").is_err());
    assert!(word_parse!(r"\u{}").is_err());
    assert!(word_parse!(r"\u{1234567}").is_err());
}

#[test]
fn quoted_string_test() {
    macro_rules! word_parse {
        ($input: expr) => {
            word_parse(concat!("\'", $input, "\'"))
        };
    };
    assert_eq!(word_parse!("hello"), Ok(literal("hello")));
    assert_eq!(word_parse!("hello world"), Ok(literal("hello world")));
    assert_eq!(word_parse!(""), Ok(literal("")));
    assert_eq!(word_parse!(r"hello\nworld"), Ok(literal("hello\nworld")));
    assert_eq!(word_parse!(r"hello\tworld"), Ok(literal("hello\tworld")));
    assert_eq!(word_parse!(r"hello\rworld"), Ok(literal("hello\rworld")));
    assert_eq!(word_parse!(r"hello\0world"), Ok(literal("hello\0world")));
    assert_eq!(word_parse!(r"hello\\world"), Ok(literal("hello\\world")));
    assert_eq!(word_parse!("say \\\"hi\\\""), Ok(literal("say \"hi\"")));
    assert_eq!(word_parse!(r"it\'s"), Ok(literal("it\'s")));
    assert_eq!(word_parse!(r"\u{41}"), Ok(literal("\u{41}")));
    assert_eq!(word_parse!(r"\u{3042}"), Ok(literal("\u{3042}")));
    assert_eq!(word_parse!(r"\u{1F600}"), Ok(literal("\u{1F600}")));
    assert_eq!(
        word_parse!(r"line1\nline2\nline3"),
        Ok(literal("line1\nline2\nline3"))
    );
}

#[test]
fn raw_string_test() {
    macro_rules! word_parse {
        (0, $input: expr) => {
            word_parse(concat!("r\"", $input, "\""))
        };
        (1, $input: expr) => {
            word_parse(concat!("r#\"", $input, "\"#"))
        };
        (2, $input: expr) => {
            word_parse(concat!("r##\"", $input, "\"##"))
        };
        (3, $input: expr) => {
            word_parse(concat!("r###\"", $input, "\"###"))
        };
    };
    assert_eq!(word_parse!(0, "hello"), Ok(literal("hello")));
    assert_eq!(word_parse!(0, "hello world"), Ok(literal("hello world")));
    assert_eq!(word_parse!(0, ""), Ok(literal("")));
    assert_eq!(
        word_parse!(0, r"hello\nworld"),
        Ok(literal(r"hello\nworld"))
    );
    assert_eq!(word_parse!(1, "hello"), Ok(literal("hello")));
    assert_eq!(
        word_parse!(1, "say \"hello\""),
        Ok(literal("say \"hello\""))
    );
    assert_eq!(word_parse!(2, "contains \"#"), Ok(literal("contains \"#")));
    assert_eq!(word_parse!(3, "a\"##b"), Ok(literal("a\"##b")));
}

#[test]
fn path_string_test() {
    let literal = |input| path_literal(input);
    macro_rules! word_parse {
        (0, $input: expr) => {
            word_parse(concat!("p\"", $input, "\""))
        };
        (1, $input: expr) => {
            word_parse(concat!("p#\"", $input, "\"#"))
        };
        (2, $input: expr) => {
            word_parse(concat!("p##\"", $input, "\"##"))
        };
        (3, $input: expr) => {
            word_parse(concat!("p###\"", $input, "\"###"))
        };
    };
    assert_eq!(word_parse!(0, "hello"), Ok(literal("hello")));
    assert_eq!(word_parse!(0, ""), Ok(literal("")));
    assert_eq!(word_parse!(0, "~/Documents"), Ok(literal("~/Documents")));
    assert_eq!(word_parse!(0, "~user/files"), Ok(literal("~user/files")));
    assert_eq!(
        word_parse!(0, r"hello\nworld"),
        Ok(literal(r"hello\nworld"))
    );
    assert_eq!(word_parse!(1, r"C:\Users\*"), Ok(literal(r"C:\Users\*")));
    assert_eq!(word_parse!(2, "path\"#here"), Ok(literal("path\"#here")));
}

#[test]
fn unquoted_string_test() {
    assert_eq!(word_parse("hello"), Ok(literal("hello")));
    assert_eq!(word_parse("hello-world"), Ok(literal("hello-world")));
    assert_eq!(
        word_parse("/usr/bin/python"),
        Ok(literal("/usr/bin/python"))
    );
    assert_eq!(word_parse("file.txt"), Ok(literal("file.txt")));
    assert_eq!(word_peek("hello world"), Ok((" world", literal("hello"))));
    assert_eq!(word_peek("hello　world"), Ok(("　world", literal("hello"))));
    assert_eq!(word_parse("hello#comment"), Ok(literal("hello#comment")));
    assert_eq!(word_parse("hello?world"), Ok(literal("hello?world")));
    assert_eq!(word_parse("hello!world"), Ok(literal("hello!world")));
    assert_eq!(word_parse("hello$world"), Ok(literal("hello$world")));
    assert_eq!(word_peek("hello|world"), Ok(("|world", literal("hello"))));
    assert_eq!(word_peek("hello>out"), Ok((">out", literal("hello"))));
    assert_eq!(word_peek("hello<in"), Ok(("<in", literal("hello"))));
    assert_eq!(word_peek("hello;next"), Ok((";next", literal("hello"))));
    assert_eq!(word_peek("hello&bg"), Ok(("&bg", literal("hello"))));
    assert_eq!(word_peek("hello(expr)"), Ok(("(expr)", literal("hello"))));
    assert_eq!(word_peek("hello{group}"), Ok(("{group}", literal("hello"))));
    assert_eq!(word_parse("rm"), Ok(literal("rm")));
    assert_eq!(word_parse("python"), Ok(literal("python")));
    assert_eq!(word_parse("r"), Ok(literal("r")));
    assert_eq!(word_parse("p"), Ok(literal("p")));
    assert_eq!(word_parse("rust"), Ok(literal("rust")));
    assert_eq!(word_parse("path"), Ok(literal("path")));
}
#[test]
fn var_test() {
    // 有効な変数名
    assert_eq!(word_parse("$PATH"), Ok(env_var("PATH")));
    assert_eq!(word_parse("$path"), Ok(env_var("path")));
    assert_eq!(word_parse("$x"), Ok(env_var("x")));
    assert_eq!(word_parse("$X"), Ok(env_var("X")));
    assert_eq!(word_parse("$my_var"), Ok(env_var("my_var")));
    assert_eq!(word_parse("$myVar"), Ok(env_var("myVar")));
    assert_eq!(word_parse("$var123"), Ok(env_var("var123")));
    assert_eq!(word_parse("$_private"), Ok(env_var("_private")));
    assert_eq!(word_parse("$__double"), Ok(env_var("__double")));
    assert_eq!(word_parse("$_123"), Ok(env_var("_123")));
    assert_eq!(word_parse("$変数"), Ok(env_var("変数")));
    assert_eq!(word_parse("$カウンタ"), Ok(env_var("カウンタ")));
    assert_eq!(word_parse("$あいう"), Ok(env_var("あいう")));
    assert_eq!(word_parse("$café"), Ok(env_var("café")));
    assert_eq!(word_parse("%PATH"), Ok(shell_var("PATH")));
    assert_eq!(word_parse("%path"), Ok(shell_var("path")));
    assert_eq!(word_parse("%x"), Ok(shell_var("x")));
    assert_eq!(word_parse("%X"), Ok(shell_var("X")));
    assert_eq!(word_parse("%my_var"), Ok(shell_var("my_var")));
    assert_eq!(word_parse("%myVar"), Ok(shell_var("myVar")));
    assert_eq!(word_parse("%var123"), Ok(shell_var("var123")));
    assert_eq!(word_parse("%_private"), Ok(shell_var("_private")));
    assert_eq!(word_parse("%__double"), Ok(shell_var("__double")));
    assert_eq!(word_parse("%_123"), Ok(shell_var("_123")));
    assert_eq!(word_parse("%変数"), Ok(shell_var("変数")));
    assert_eq!(word_parse("%カウンタ"), Ok(shell_var("カウンタ")));
    assert_eq!(word_parse("%あいう"), Ok(shell_var("あいう")));
    assert_eq!(word_parse("%café"), Ok(shell_var("café")));

    // 無効な変数名
    assert!(word_parse("$123").is_err());
    assert!(word_parse("$2var").is_err());
    assert!(word_parse("$_").is_err());
    assert!(word_parse("$").is_err());
    assert!(word_parse("$-var").is_err());
    assert!(word_parse("%123").is_err());
    assert!(word_parse("%2var").is_err());
    assert!(word_parse("%_").is_err());
    assert!(word_parse("%").is_err());
    assert!(word_parse("%-var").is_err());

    // 特殊変数
    assert_eq!(word_parse("$?"), Ok(special_var(SpecialVar::ExitStatus)));
    assert_eq!(word_parse("$$"), Ok(special_var(SpecialVar::Pid)));
    assert_eq!(word_parse("$!"), Ok(special_var(SpecialVar::BackgroundPid)));
    assert_eq!(word_parse("$@"), Ok(special_var(SpecialVar::ShellName)));
    assert!(word_parse("$#").is_err());
    assert!(word_parse("$*").is_err());
    assert!(word_parse("$%").is_err());

    // 変数の境目
    assert_eq!(word_peek("$PATH/bin"), Ok(("/bin", env_var("PATH"))));
    assert_eq!(word_peek("$var-suffix"), Ok(("-suffix", env_var("var"))));
    assert_eq!(word_peek("$var.txt"), Ok((".txt", env_var("var"))));
    assert_eq!(word_peek("$var:value"), Ok((":value", env_var("var"))));
    assert_eq!(word_peek("$var=value"), Ok(("=value", env_var("var"))));
    assert_eq!(word_peek("$a b"), Ok((" b", env_var("a"))));
    assert_eq!(word_peek("$var#comment"), Ok(("#comment", env_var("var"))));
    assert_eq!(word_peek("$var$other"), Ok(("$other", env_var("var"))));
    assert_eq!(word_peek("$var%other"), Ok(("%other", env_var("var"))));
    assert_eq!(word_peek("%PATH/bin"), Ok(("/bin", shell_var("PATH"))));
    assert_eq!(word_peek("%var-suffix"), Ok(("-suffix", shell_var("var"))));
    assert_eq!(word_peek("%var.txt"), Ok((".txt", shell_var("var"))));
    assert_eq!(word_peek("%var:value"), Ok((":value", shell_var("var"))));
    assert_eq!(word_peek("%var=value"), Ok(("=value", shell_var("var"))));
    assert_eq!(word_peek("%a b"), Ok((" b", shell_var("a"))));
    assert_eq!(
        word_peek("%var#comment"),
        Ok(("#comment", shell_var("var")))
    );
    assert_eq!(word_peek("%var$other"), Ok(("$other", shell_var("var"))));
    assert_eq!(word_peek("%var%other"), Ok(("%other", shell_var("var"))));
    assert_eq!(
        word_peek("$?var"),
        Ok(("var", special_var(SpecialVar::ExitStatus)))
    );
    assert_eq!(
        word_peek("$$abc"),
        Ok(("abc", special_var(SpecialVar::Pid)))
    );
    assert_eq!(
        word_peek("$!foo"),
        Ok(("foo", special_var(SpecialVar::BackgroundPid)))
    );
    assert_eq!(
        word_peek("$@var"),
        Ok(("var", special_var(SpecialVar::ShellName)))
    );
}

#[test]
fn comment_test() {
    let comment_only = |comment: &str| ShellCommand {
        commands: vec![],
        comment: Some(comment.to_string()),
    };

    // コメントのみ
    assert_eq!(shell_parse("# comment"), Ok(comment_only(" comment")));
    assert_eq!(shell_parse("#comment"), Ok(comment_only("comment")));
    assert_eq!(shell_parse("#"), Ok(comment_only("")));
    assert_eq!(shell_parse("# "), Ok(comment_only(" ")));
    assert_eq!(
        shell_parse("#  multiple  spaces"),
        Ok(comment_only("  multiple  spaces"))
    );
    assert_eq!(
        shell_parse("# 日本語コメント"),
        Ok(comment_only(" 日本語コメント"))
    );

    // インラインコメント
    assert_eq!(
        shell_parse("echo hello # comment"),
        Ok(shell("echo", &["hello"], Some(" comment")))
    );
    assert_eq!(
        shell_parse("echo hello #comment"),
        Ok(shell("echo", &["hello"], Some("comment")))
    );
    assert_eq!(
        shell_parse("echo hello #"),
        Ok(shell("echo", &["hello"], Some("")))
    );
    assert_eq!(
        shell_parse("echo hello  #  spaced"),
        Ok(shell("echo", &["hello"], Some("  spaced")))
    );
    assert_eq!(
        shell_parse("echo # comment"),
        Ok(shell("echo", &[], Some(" comment")))
    );
    assert_eq!(
        shell_parse("ls -la /tmp # list files"),
        Ok(shell("ls", &["-la", "/tmp"], Some(" list files")))
    );

    // コメントなし
    assert_eq!(
        shell_parse("echo hello"),
        Ok(shell("echo", &["hello"], None))
    );
    assert_eq!(shell_parse("echo"), Ok(shell("echo", &[], None)));
    assert_eq!(shell_parse("ls -la"), Ok(shell("ls", &["-la"], None)));

    // 空白のみ
    let whitespace = Ok(ShellCommand {
        commands: vec![],
        comment: None,
    });
    assert_eq!(shell_parse(""), whitespace);
    assert_eq!(shell_parse(" "), whitespace);
    assert_eq!(shell_parse("   "), whitespace);
    assert_eq!(shell_parse("\t"), whitespace);
    assert_eq!(shell_parse("　"), whitespace);

    // #をコメントとして扱わないケース
    assert_eq!(
        shell_parse("echo hello#world"),
        Ok(shell("echo", &["hello#world"], None))
    );
    assert_eq!(
        shell_parse("echo#comment"),
        Ok(shell("echo#comment", &[], None))
    );
    assert_eq!(shell_parse("file#1"), Ok(shell("file#1", &[], None)));
    assert_eq!(
        shell_parse("echo a#b c#d"),
        Ok(shell("echo", &["a#b", "c#d"], None))
    );
    assert_eq!(
        shell_parse("echo a#b #comment"),
        Ok(shell("echo", &["a#b"], Some("comment")))
    );

    // クォート内の#
    assert_eq!(
        shell_parse("echo \"hello # world\""),
        Ok(shell("echo", &["hello # world"], None))
    );
    assert_eq!(
        shell_parse("echo \'hello # world\'"),
        Ok(shell("echo", &["hello # world"], None))
    );
    assert_eq!(
        shell_parse("echo \"a#b\" #comment"),
        Ok(shell("echo", &["a#b"], Some("comment")))
    );
    assert_eq!(
        shell_parse(r#"echo r"C:\#path""#),
        Ok(shell("echo", &[r"C:\#path"], None))
    );
    assert_eq!(
        shell_parse(r#"echo p"~/#dir""#),
        Ok(ShellCommand {
            commands: vec![(
                Command {
                    name: literal("echo"),
                    args: vec![path_literal("~/#dir")]
                },
                None
            )],
            comment: None
        })
    );

    // 複合ケース
    assert_eq!(
        shell_parse(r#"echo "hello" world # comment"#),
        Ok(shell("echo", &["hello", "world"], Some(" comment")))
    );
    assert_eq!(
        shell_parse(r#"echo "a # b" c # real comment"#),
        Ok(shell("echo", &["a # b", "c"], Some(" real comment")))
    );
    assert_eq!(
        shell_parse(r#"cmd arg1 "arg #2" arg3 # end"#),
        Ok(shell("cmd", &["arg1", "arg #2", "arg3"], Some(" end")))
    );
    assert_eq!(
        shell_parse("echo $PATH # show path"),
        Ok(ShellCommand {
            commands: vec![(
                Command {
                    name: literal("echo"),
                    args: vec![env_var("PATH")]
                },
                None
            )],
            comment: Some(" show path".to_string())
        })
    );
    assert_eq!(
        shell_parse("echo %var # shell var"),
        Ok(ShellCommand {
            commands: vec![(
                Command {
                    name: literal("echo"),
                    args: vec![shell_var("var")]
                },
                None
            )],
            comment: Some(" shell var".to_string())
        })
    );
}
