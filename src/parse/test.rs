use super::*;

fn cmd(name: &str, args: &[&str]) -> Command {
    Command {
        name: name.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
    }
}

#[test]
fn command_test() {
    // 基本
    assert_eq!(command.parse("ls"), Ok(cmd("ls", &[])));
    assert_eq!(command.parse("ls -la"), Ok(cmd("ls", &["-la"])));
    assert_eq!(
        command.parse("echo hello world"),
        Ok(cmd("echo", &["hello", "world"]))
    );
    assert_eq!(
        command.parse("git commit -m message"),
        Ok(cmd("git", &["commit", "-m", "message"]))
    );

    // 空白
    assert_eq!(command.parse(" ls "), Ok(cmd("ls", &[])));
    assert_eq!(
        command.parse("echo    hello     world"),
        Ok(cmd("echo", &["hello", "world"]))
    );
    assert_eq!(command.parse("ls -l -a"), Ok(cmd("ls", &["-l", "-a"])));

    // ダブルクォート
    assert_eq!(
        command.parse(r#"echo "hello world""#),
        Ok(cmd("echo", &["hello world"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello""#),
        Ok(cmd("echo", &["hello"]))
    );
    assert_eq!(command.parse(r#"echo """#), Ok(cmd("echo", &[""])));
    assert_eq!(
        command.parse(r#"git commit -m "fix bug""#),
        Ok(cmd("git", &["commit", "-m", "fix bug"]))
    );
    assert_eq!(
        command.parse(r#"echo "a" "b" "c""#),
        Ok(cmd("echo", &["a", "b", "c"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello   world""#),
        Ok(cmd("echo", &["hello   world"]))
    );

    // シングルクォート
    assert_eq!(
        command.parse("echo 'hello world'"),
        Ok(cmd("echo", &["hello world"]))
    );
    assert_eq!(command.parse("echo 'hello'"), Ok(cmd("echo", &["hello"])));
    assert_eq!(command.parse("echo ''"), Ok(cmd("echo", &[""])));
    assert_eq!(
        command.parse("echo 'a' 'b' 'c'"),
        Ok(cmd("echo", &["a", "b", "c"]))
    );

    // クォートの混合
    assert_eq!(
        command.parse(r#"echo "hello" 'world'"#),
        Ok(cmd("echo", &["hello", "world"]))
    );
    assert_eq!(command.parse(r#"echo "it's""#), Ok(cmd("echo", &["it's"])));
    assert_eq!(
        command.parse(r#"echo 'say "hello"'"#),
        Ok(cmd("echo", &[r#"say "hello""#]))
    );

    // クォートの連結
    assert_eq!(
        command.parse(r#"echo hello"world""#),
        Ok(cmd("echo", &["helloworld"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello"world"#),
        Ok(cmd("echo", &["helloworld"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello"'world'"#),
        Ok(cmd("echo", &["helloworld"]))
    );
    assert_eq!(
        command.parse(r#"echo he"ll"o"#),
        Ok(cmd("echo", &["hello"]))
    );
    assert_eq!(
        command.parse(r#"echo 'hel'"lo""#),
        Ok(cmd("echo", &["hello"]))
    );

    // エスケープ
    assert_eq!(
        command.parse(r#"echo hello\ world"#),
        Ok(cmd("echo", &["hello world"]))
    );
    assert_eq!(
        command.parse(r#"echo hello\\world"#),
        Ok(cmd("echo", &["hello\\world"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello\"world""#),
        Ok(cmd("echo", &["hello\"world"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello\\world""#),
        Ok(cmd("echo", &["hello\\world"]))
    );
    assert_eq!(
        command.parse(r#"echo 'hello\world'"#),
        Ok(cmd("echo", &["hello\\world"]))
    );
    assert_eq!(
        command.parse(r#"echo 'hello\'"#),
        Ok(cmd("echo", &["hello\\"]))
    );

    // 特殊な文字を含むケース
    assert_eq!(
        command.parse("echo hello-world"),
        Ok(cmd("echo", &["hello-world"]))
    );
    assert_eq!(
        command.parse("echo hello_world"),
        Ok(cmd("echo", &["hello_world"]))
    );
    assert_eq!(
        command.parse("echo hello.world"),
        Ok(cmd("echo", &["hello.world"]))
    );
    assert_eq!(
        command.parse("echo hello/world"),
        Ok(cmd("echo", &["hello/world"]))
    );
    assert_eq!(command.parse("echo -n"), Ok(cmd("echo", &["-n"])));
    assert_eq!(command.parse("./script.sh"), Ok(cmd("./script.sh", &[])));
    assert_eq!(command.parse("/usr/bin/env"), Ok(cmd("/usr/bin/env", &[])));

    // コーナーケース
    assert!(command.parse("").is_err());
    assert!(command.parse(" ").is_err());
    assert_eq!(command.parse("echo"), Ok(cmd("echo", &[])));
    assert_eq!(command.parse(r#""echo""#), Ok(cmd("echo", &[])));
    assert_eq!(
        command.parse(r#""echo" "hello""#),
        Ok(cmd("echo", &["hello"]))
    );
    assert_eq!(
        command.parse(r#"'echo' 'hello'"#),
        Ok(cmd("echo", &["hello"]))
    );
    assert!(command.parse(r#"echo "hello"#).is_err());
    assert!(command.parse(r#"echo 'hello"#).is_err());
    assert_eq!(command.parse(r"echo \"), Ok(cmd("echo", &["\\"])));

    // タブ文字
    assert_eq!(command.parse("echo\thello"), Ok(cmd("echo", &["hello"])));
    assert_eq!(
        command.parse("echo \"\thello\""),
        Ok(cmd("echo", &["\thello"]))
    );

    // 日本語
    assert_eq!(
        command.parse("echo こんにちは"),
        Ok(cmd("echo", &["こんにちは"]))
    );
    assert_eq!(
        command.parse(r#"echo "こんにちは 世界""#),
        Ok(cmd("echo", &["こんにちは 世界"]))
    );

    // 全角空白
    assert_eq!(command.parse("echo　hello"), Ok(cmd("echo　hello", &[])));
    assert_eq!(
        command.parse("echo hello　world"),
        Ok(cmd("echo", &["hello　world"]))
    );
    assert_eq!(
        command.parse(r#"echo "hello　world""#),
        Ok(cmd("echo", &["hello　world"]))
    );
    assert_eq!(
        command.parse("echo　　hello"),
        Ok(cmd("echo　　hello", &[]))
    );
    assert_eq!(command.parse("echo 　hello"), Ok(cmd("echo", &["　hello"])));
    assert_eq!(command.parse("echo　 hello"), Ok(cmd("echo　", &["hello"])));
}
