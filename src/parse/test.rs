use super::*;

fn literal(literal: &str) -> Word {
    Word::Literal(literal.to_string())
}
fn path_literal(path_literal: &str) -> Word {
    Word::PathLiteral(path_literal.to_string())
}
fn env_var(var: &str) -> Word {
    Word::EnvVar(var.to_string())
}
fn shell_var(var: &str) -> Word {
    Word::ShellVar(var.to_string())
}
fn shell(command: &str, args: &[&str], comment: Option<&str>) -> ShellCommand {
    ShellCommand {
        commands: vec![(
            Command {
                name: literal(command),
                args: args.iter().copied().map(literal).collect(),
            },
            None,
        )],
        comment: comment.map(str::to_string),
    }
}
fn shell_parse(
    input: &str,
) -> Result<
    ShellCommand,
    winnow::error::ParseError<&str, winnow::error::ContextError>,
> {
    super::shell_command.parse(input)
}
fn word_parse(
    input: &str,
) -> Result<Word, winnow::error::ParseError<&str, winnow::error::ContextError>>
{
    super::word.parse(input)
}
fn word_peek(
    input: &str,
) -> Result<(&str, Word), winnow::error::ErrMode<winnow::error::ContextError>> {
    super::word.parse_peek(input)
}

#[test]
fn double_quoted_string_test() {
    macro_rules! word_parse {
        ($input: expr) => {
            word_parse(concat!("\"", $input, "\""))
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
