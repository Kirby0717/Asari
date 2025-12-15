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
fn shell(
    command: SpannedInput,
    args: &[SpannedInput],
    comment: Option<&str>,
) -> ShellCommand {
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
fn parse_error(kind: ParseErrorKind, span: usize) -> ParseError {
    ParseError { kind, span }
}

fn shell_parse(
    input: &str,
) -> Result<
    ShellCommand,
    winnow::error::ParseError<LocatingSlice<&str>, ParseError>,
> {
    super::shell_command.parse(LocatingSlice::new(input))
}
fn word_parse(
    input: &str,
) -> Result<
    Spanned<Word>,
    winnow::error::ParseError<LocatingSlice<&str>, ParseError>,
> {
    super::word.parse(LocatingSlice::new(input))
}
fn word_peek(
    input: &str,
) -> Result<(&str, Spanned<Word>), winnow::error::ErrMode<ParseError>> {
    super::word
        .parse_peek(LocatingSlice::new(input))
        .map(|(rest, result)| (*rest.as_ref(), result))
}

#[test]
fn double_quoted_string_test() {
    macro_rules! word_parse {
        ($input: expr) => {
            word_parse(concat!("\"", $input, "\""))
        };
    };
    assert_eq!(word_parse!("hello"), Ok(literal(("hello", 0..7))));
    assert_eq!(
        word_parse!("hello world"),
        Ok(literal(("hello world", 0..13)))
    );
    assert_eq!(word_parse!(""), Ok(literal(("", 0..2))));
    assert_eq!(
        word_parse!(r"hello\nworld"),
        Ok(literal(("hello\nworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\tworld"),
        Ok(literal(("hello\tworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\rworld"),
        Ok(literal(("hello\rworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\0world"),
        Ok(literal(("hello\0world", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\\world"),
        Ok(literal(("hello\\world", 0..14)))
    );
    assert_eq!(
        word_parse!("say \\\"hi\\\""),
        Ok(literal(("say \"hi\"", 0..12)))
    );
    assert_eq!(word_parse!(r"it\'s"), Ok(literal(("it\'s", 0..7))));
    assert_eq!(word_parse!(r"\u{41}"), Ok(literal(("\u{41}", 0..8))));
    assert_eq!(word_parse!(r"\u{3042}"), Ok(literal(("\u{3042}", 0..10))));
    assert_eq!(word_parse!(r"\u{1F600}"), Ok(literal(("\u{1F600}", 0..11))));
    assert_eq!(
        word_parse!(r"line1\nline2\nline3"),
        Ok(literal(("line1\nline2\nline3", 0..21)))
    );

    assert_eq!(
        word_parse!(r"\x").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::UnrecognizedEscape('x'), 2)
    );
    assert_eq!(
        word_parse!(r"\u{GGGG}").unwrap_err().into_inner().kind,
        ParseErrorKind::ParseHexError(
            u32::from_str_radix("GGGG", 16).unwrap_err()
        )
    );
    assert_eq!(
        word_parse!(r"\u{110000}").unwrap_err().into_inner(),
        parse_error(
            ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::InvalidUnicode
            ),
            4
        )
    );
    assert_eq!(
        word_parse!(r"\u{}").unwrap_err().into_inner().kind,
        ParseErrorKind::ParseHexError(u32::from_str_radix("", 16).unwrap_err())
    );
    assert_eq!(
        word_parse!(r"\u{1234567}").unwrap_err().into_inner(),
        parse_error(
            ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::InvalidUnicode
            ),
            4
        )
    );
}

#[test]
fn quoted_string_test() {
    macro_rules! word_parse {
        ($input: expr) => {
            word_parse(concat!("\'", $input, "\'"))
        };
    };
    assert_eq!(word_parse!("hello"), Ok(literal(("hello", 0..7))));
    assert_eq!(
        word_parse!("hello world"),
        Ok(literal(("hello world", 0..13)))
    );
    assert_eq!(word_parse!(""), Ok(literal(("", 0..2))));
    assert_eq!(
        word_parse!(r"hello\nworld"),
        Ok(literal(("hello\nworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\tworld"),
        Ok(literal(("hello\tworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\rworld"),
        Ok(literal(("hello\rworld", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\0world"),
        Ok(literal(("hello\0world", 0..14)))
    );
    assert_eq!(
        word_parse!(r"hello\\world"),
        Ok(literal(("hello\\world", 0..14)))
    );
    assert_eq!(
        word_parse!("say \\\"hi\\\""),
        Ok(literal(("say \"hi\"", 0..12)))
    );
    assert_eq!(word_parse!(r"it\'s"), Ok(literal(("it\'s", 0..7))));
    assert_eq!(word_parse!(r"\u{41}"), Ok(literal(("\u{41}", 0..8))));
    assert_eq!(word_parse!(r"\u{3042}"), Ok(literal(("\u{3042}", 0..10))));
    assert_eq!(word_parse!(r"\u{1F600}"), Ok(literal(("\u{1F600}", 0..11))));
    assert_eq!(
        word_parse!(r"line1\nline2\nline3"),
        Ok(literal(("line1\nline2\nline3", 0..21)))
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
    assert_eq!(word_parse!(0, "hello"), Ok(literal(("hello", 0..8))));
    assert_eq!(
        word_parse!(0, "hello world"),
        Ok(literal(("hello world", 0..14)))
    );
    assert_eq!(word_parse!(0, ""), Ok(literal(("", 0..3))));
    assert_eq!(
        word_parse!(0, r"hello\nworld"),
        Ok(literal((r"hello\nworld", 0..15)))
    );
    assert_eq!(word_parse!(1, "hello"), Ok(literal(("hello", 0..10))));
    assert_eq!(
        word_parse!(1, "say \"hello\""),
        Ok(literal(("say \"hello\"", 0..16)))
    );
    assert_eq!(
        word_parse!(2, "contains \"#"),
        Ok(literal(("contains \"#", 0..18)))
    );
    assert_eq!(word_parse!(3, "a\"##b"), Ok(literal(("a\"##b", 0..14))));
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
    assert_eq!(word_parse!(0, "hello"), Ok(literal(("hello", 0..8))));
    assert_eq!(word_parse!(0, ""), Ok(literal(("", 0..3))));
    assert_eq!(
        word_parse!(0, "~/Documents"),
        Ok(literal(("~/Documents", 0..14)))
    );
    assert_eq!(
        word_parse!(0, "~user/files"),
        Ok(literal(("~user/files", 0..14)))
    );
    assert_eq!(
        word_parse!(0, r"hello\nworld"),
        Ok(literal((r"hello\nworld", 0..15)))
    );
    assert_eq!(
        word_parse!(1, r"C:\Users\*"),
        Ok(literal((r"C:\Users\*", 0..15)))
    );
    assert_eq!(
        word_parse!(2, "path\"#here"),
        Ok(literal(("path\"#here", 0..17)))
    );
}

#[test]
fn unquoted_string_test() {
    assert_eq!(word_parse("hello"), Ok(literal(("hello", 0..5))));
    assert_eq!(
        word_parse("hello-world"),
        Ok(literal(("hello-world", 0..11)))
    );
    assert_eq!(
        word_parse("/usr/bin/python"),
        Ok(literal(("/usr/bin/python", 0..15)))
    );
    assert_eq!(word_parse("file.txt"), Ok(literal(("file.txt", 0..8))));
    assert_eq!(
        word_peek("hello world"),
        Ok((" world", literal(("hello", 0..5))))
    );
    assert_eq!(
        word_peek("hello　world"),
        Ok(("　world", literal(("hello", 0..5))))
    );
    assert_eq!(
        word_parse("hello#comment"),
        Ok(literal(("hello#comment", 0..13)))
    );
    assert_eq!(
        word_parse("hello?world"),
        Ok(literal(("hello?world", 0..11)))
    );
    assert_eq!(
        word_parse("hello!world"),
        Ok(literal(("hello!world", 0..11)))
    );
    assert_eq!(
        word_parse("hello$world"),
        Ok(literal(("hello$world", 0..11)))
    );
    assert_eq!(
        word_peek("hello|world"),
        Ok(("|world", literal(("hello", 0..5))))
    );
    assert_eq!(
        word_peek("hello>out"),
        Ok((">out", literal(("hello", 0..5))))
    );
    assert_eq!(word_peek("hello<in"), Ok(("<in", literal(("hello", 0..5)))));
    assert_eq!(
        word_peek("hello;next"),
        Ok((";next", literal(("hello", 0..5))))
    );
    assert_eq!(word_peek("hello&bg"), Ok(("&bg", literal(("hello", 0..5)))));
    assert_eq!(
        word_peek("hello(expr)"),
        Ok(("(expr)", literal(("hello", 0..5))))
    );
    assert_eq!(
        word_peek("hello{group}"),
        Ok(("{group}", literal(("hello", 0..5))))
    );
    assert_eq!(word_parse("rm"), Ok(literal(("rm", 0..2))));
    assert_eq!(word_parse("python"), Ok(literal(("python", 0..6))));
    assert_eq!(word_parse("r"), Ok(literal(("r", 0..1))));
    assert_eq!(word_parse("p"), Ok(literal(("p", 0..1))));
    assert_eq!(word_parse("rust"), Ok(literal(("rust", 0..4))));
    assert_eq!(word_parse("path"), Ok(literal(("path", 0..4))));
}
#[test]
fn var_test() {
    // 有効な変数名
    assert_eq!(word_parse("$PATH"), Ok(env_var(("PATH", 0..5))));
    assert_eq!(word_parse("$path"), Ok(env_var(("path", 0..5))));
    assert_eq!(word_parse("$x"), Ok(env_var(("x", 0..2))));
    assert_eq!(word_parse("$X"), Ok(env_var(("X", 0..2))));
    assert_eq!(word_parse("$my_var"), Ok(env_var(("my_var", 0..7))));
    assert_eq!(word_parse("$myVar"), Ok(env_var(("myVar", 0..6))));
    assert_eq!(word_parse("$var123"), Ok(env_var(("var123", 0..7))));
    assert_eq!(word_parse("$_private"), Ok(env_var(("_private", 0..9))));
    assert_eq!(word_parse("$__double"), Ok(env_var(("__double", 0..9))));
    assert_eq!(word_parse("$_123"), Ok(env_var(("_123", 0..5))));
    assert_eq!(word_parse("$変数"), Ok(env_var(("変数", 0..7))));
    assert_eq!(word_parse("$カウンタ"), Ok(env_var(("カウンタ", 0..13))));
    assert_eq!(word_parse("$あいう"), Ok(env_var(("あいう", 0..10))));
    assert_eq!(word_parse("$café"), Ok(env_var(("café", 0..6))));
    assert_eq!(word_parse("%PATH"), Ok(shell_var(("PATH", 0..5))));
    assert_eq!(word_parse("%path"), Ok(shell_var(("path", 0..5))));
    assert_eq!(word_parse("%x"), Ok(shell_var(("x", 0..2))));
    assert_eq!(word_parse("%X"), Ok(shell_var(("X", 0..2))));
    assert_eq!(word_parse("%my_var"), Ok(shell_var(("my_var", 0..7))));
    assert_eq!(word_parse("%myVar"), Ok(shell_var(("myVar", 0..6))));
    assert_eq!(word_parse("%var123"), Ok(shell_var(("var123", 0..7))));
    assert_eq!(word_parse("%_private"), Ok(shell_var(("_private", 0..9))));
    assert_eq!(word_parse("%__double"), Ok(shell_var(("__double", 0..9))));
    assert_eq!(word_parse("%_123"), Ok(shell_var(("_123", 0..5))));
    assert_eq!(word_parse("%変数"), Ok(shell_var(("変数", 0..7))));
    assert_eq!(word_parse("%カウンタ"), Ok(shell_var(("カウンタ", 0..13))));
    assert_eq!(word_parse("%あいう"), Ok(shell_var(("あいう", 0..10))));
    assert_eq!(word_parse("%café"), Ok(shell_var(("café", 0..6))));

    // 無効な変数名
    assert_eq!(
        word_parse("$123").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("$2var").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("$_").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::InvalidIdent, 1)
    );
    assert_eq!(
        word_parse("$").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("$-var").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("%123").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("%2var").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("%_").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::InvalidIdent, 1)
    );
    assert_eq!(
        word_parse("%").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("%-var").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );

    // 特殊変数
    assert_eq!(
        word_parse("$?"),
        Ok(special_var(SpecialVar::ExitStatus, 0..2))
    );
    assert_eq!(word_parse("$$"), Ok(special_var(SpecialVar::Pid, 0..2)));
    assert_eq!(
        word_parse("$!"),
        Ok(special_var(SpecialVar::BackgroundPid, 0..2))
    );
    assert_eq!(
        word_parse("$@"),
        Ok(special_var(SpecialVar::ShellName, 0..2))
    );
    assert_eq!(
        word_parse("$#").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("$*").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );
    assert_eq!(
        word_parse("$%").unwrap_err().into_inner(),
        parse_error(ParseErrorKind::NoIdent, 1)
    );

    // 変数の境目
    assert_eq!(
        word_peek("$PATH/bin"),
        Ok(("/bin", env_var(("PATH", 0..5))))
    );
    assert_eq!(
        word_peek("$var-suffix"),
        Ok(("-suffix", env_var(("var", 0..4))))
    );
    assert_eq!(word_peek("$var.txt"), Ok((".txt", env_var(("var", 0..4)))));
    assert_eq!(
        word_peek("$var:value"),
        Ok((":value", env_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("$var=value"),
        Ok(("=value", env_var(("var", 0..4))))
    );
    assert_eq!(word_peek("$a b"), Ok((" b", env_var(("a", 0..2)))));
    assert_eq!(
        word_peek("$var#comment"),
        Ok(("#comment", env_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("$var$other"),
        Ok(("$other", env_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("$var%other"),
        Ok(("%other", env_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%PATH/bin"),
        Ok(("/bin", shell_var(("PATH", 0..5))))
    );
    assert_eq!(
        word_peek("%var-suffix"),
        Ok(("-suffix", shell_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%var.txt"),
        Ok((".txt", shell_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%var:value"),
        Ok((":value", shell_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%var=value"),
        Ok(("=value", shell_var(("var", 0..4))))
    );
    assert_eq!(word_peek("%a b"), Ok((" b", shell_var(("a", 0..2)))));
    assert_eq!(
        word_peek("%var#comment"),
        Ok(("#comment", shell_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%var$other"),
        Ok(("$other", shell_var(("var", 0..4))))
    );
    assert_eq!(
        word_peek("%var%other"),
        Ok(("%other", shell_var(("var", 0..4))))
    );

    // 特殊変数の境目
    assert_eq!(
        word_peek("$?var"),
        Ok(("var", special_var(SpecialVar::ExitStatus, 0..2)))
    );
    assert_eq!(
        word_peek("$$abc"),
        Ok(("abc", special_var(SpecialVar::Pid, 0..2)))
    );
    assert_eq!(
        word_peek("$!foo"),
        Ok(("foo", special_var(SpecialVar::BackgroundPid, 0..2)))
    );
    assert_eq!(
        word_peek("$@var"),
        Ok(("var", special_var(SpecialVar::ShellName, 0..2)))
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
        Ok(shell(("echo", 0..4), &[("hello", 5..10)], Some(" comment")))
    );
    assert_eq!(
        shell_parse("echo hello #comment"),
        Ok(shell(("echo", 0..4), &[("hello", 5..10)], Some("comment")))
    );
    assert_eq!(
        shell_parse("echo hello #"),
        Ok(shell(("echo", 0..4), &[("hello", 5..10)], Some("")))
    );
    assert_eq!(
        shell_parse("echo hello  #  spaced"),
        Ok(shell(("echo", 0..4), &[("hello", 5..10)], Some("  spaced")))
    );
    assert_eq!(
        shell_parse("echo # comment"),
        Ok(shell(("echo", 0..4), &[], Some(" comment")))
    );
    assert_eq!(
        shell_parse("ls -la /tmp # list files"),
        Ok(shell(
            ("ls", 0..2),
            &[("-la", 3..6), ("/tmp", 7..11)],
            Some(" list files")
        ))
    );

    // コメントなし
    assert_eq!(
        shell_parse("echo hello"),
        Ok(shell(("echo", 0..4), &[("hello", 5..10)], None))
    );
    assert_eq!(shell_parse("echo"), Ok(shell(("echo", 0..4), &[], None)));
    assert_eq!(
        shell_parse("ls -la"),
        Ok(shell(("ls", 0..2), &[("-la", 3..6)], None))
    );

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
        Ok(shell(("echo", 0..4), &[("hello#world", 5..16)], None))
    );
    assert_eq!(
        shell_parse("echo#comment"),
        Ok(shell(("echo#comment", 0..12), &[], None))
    );
    assert_eq!(
        shell_parse("file#1"),
        Ok(shell(("file#1", 0..6), &[], None))
    );
    assert_eq!(
        shell_parse("echo a#b c#d"),
        Ok(shell(
            ("echo", 0..4),
            &[("a#b", 5..8), ("c#d", 9..12)],
            None
        ))
    );
    assert_eq!(
        shell_parse("echo a#b #comment"),
        Ok(shell(("echo", 0..4), &[("a#b", 5..8)], Some("comment")))
    );

    // クォート内の#
    assert_eq!(
        shell_parse("echo \"hello # world\""),
        Ok(shell(("echo", 0..4), &[("hello # world", 5..20)], None))
    );
    assert_eq!(
        shell_parse("echo \'hello # world\'"),
        Ok(shell(("echo", 0..4), &[("hello # world", 5..20)], None))
    );
    assert_eq!(
        shell_parse("echo \"a#b\" #comment"),
        Ok(shell(("echo", 0..4), &[("a#b", 5..10)], Some("comment")))
    );
    assert_eq!(
        shell_parse(r#"echo r"C:\#path""#),
        Ok(shell(("echo", 0..4), &[(r"C:\#path", 5..16)], None))
    );
    assert_eq!(
        shell_parse(r#"echo p"~/#dir""#),
        Ok(ShellCommand {
            commands: vec![(
                Command {
                    name: literal(("echo", 0..4)),
                    args: vec![path_literal(("~/#dir", 5..14))]
                },
                None
            )],
            comment: None
        })
    );

    // 複合ケース
    assert_eq!(
        shell_parse(r#"echo "hello" world # comment"#),
        Ok(shell(
            ("echo", 0..4),
            &[("hello", 5..12), ("world", 13..18)],
            Some(" comment")
        ))
    );
    assert_eq!(
        shell_parse(r#"echo "a # b" c # real comment"#),
        Ok(shell(
            ("echo", 0..4),
            &[("a # b", 5..12), ("c", 13..14)],
            Some(" real comment")
        ))
    );
    assert_eq!(
        shell_parse(r#"cmd arg1 "arg #2" arg3 # end"#),
        Ok(shell(
            ("cmd", 0..3),
            &[("arg1", 4..8), ("arg #2", 9..17), ("arg3", 18..22)],
            Some(" end")
        ))
    );
    assert_eq!(
        shell_parse("echo $PATH # show path"),
        Ok(ShellCommand {
            commands: vec![(
                Command {
                    name: literal(("echo", 0..4)),
                    args: vec![env_var(("PATH", 5..10))]
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
                    name: literal(("echo", 0..4)),
                    args: vec![shell_var(("var", 5..9))]
                },
                None
            )],
            comment: Some(" shell var".to_string())
        })
    );
}
