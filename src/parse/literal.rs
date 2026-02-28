use super::*;

use winnow::stream::Location;

pub fn unicode_number(input: &mut Input) -> ModalResult<char> {
    use UnicodeEscapeError::*;
    take_until(0.., '}')
        .map_err_with_span(|()| NoCloseBrace)
        .try_map_with_span(|input| {
            let code = u32::from_str_radix(input, 16).map_err(InvalidHex)?;
            char::from_u32(code).ok_or(InvalidCodePoint)
        })
        .cut()
        .parse_next(input)
}
pub fn unicode_escape_char(input: &mut Input) -> ModalResult<char> {
    use UnicodeEscapeError::*;
    let _ = 'u'.parse_next(input)?;
    let _ = '{'.map_err_with_span(|()| NoOpenBrace).parse_next(input)?;
    let c = unicode_number.cut().parse_next(input)?;
    let _ = '}'
        .map_err_with_span(|()| NoCloseBrace)
        .cut()
        .parse_next(input)?;
    Ok(c)
}
pub fn escape_char(input: &mut Input) -> ModalResult<char> {
    preceded(
        '\\',
        dispatch!(peek(any);
            'n' => any.value('\n'),
            'r' => any.value('\r'),
            't' => any.value('\t'),
            'u' => unicode_escape_char.cut(),
            '\\' => any.value('\\'),
            '\"' => any.value('\"'),
            '\'' => any.value('\''),
            '0' => any.value('\0'),
            c => fail.map_err_with_span(|()| {
                LiteralError::UnrecognizedEscape(c)
            }).cut(),
        ),
    )
    .parse_next(input)
}

pub fn quoted_string(input: &mut Input) -> ModalResult<String> {
    let begin = input.current_token_start();
    let _ = '\''.parse_next(input)?;
    let end = input.previous_token_end();

    let s = repeat(0.., alt((escape_char, any.verify(|c| *c != '\''))))
        .parse_next(input)?;

    let _ = '\''
        .map_err_at(|()| LiteralError::UnclosedQuote, begin..end)
        .cut()
        .parse_next(input)?;
    Ok(s)
}
pub fn double_quoted_string(input: &mut Input) -> ModalResult<String> {
    let begin = input.current_token_start();
    let _ = '"'.parse_next(input)?;
    let end = input.previous_token_end();

    let s = repeat(0.., alt((escape_char, any.verify(|c| *c != '"'))))
        .parse_next(input)?;

    let _ = '"'
        .map_err_at(|()| LiteralError::UnclosedDoubleQuote, begin..end)
        .cut()
        .parse_next(input)?;
    Ok(s)
}
pub fn raw_string(input: &mut Input) -> ModalResult<String> {
    let begin = input.current_token_start();
    let _ = 'r'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let end = input.previous_token_end();

    let s = take_until(0.., delimiter.as_str())
        .map_err_at(|()| LiteralError::UnclosedRawString, begin..end)
        .cut()
        .parse_next(input)?;

    let _ = delimiter.as_str().parse_next(input)?;
    Ok(s.to_string())
}
pub fn path_string(input: &mut Input) -> ModalResult<String> {
    let begin = input.current_token_start();
    let _ = 'p'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let end = input.previous_token_end();

    let s = take_until(0.., delimiter.as_str())
        .map_err_at(|()| LiteralError::UnclosedPathString, begin..end)
        .cut()
        .parse_next(input)?;

    let _ = delimiter.as_str().parse_next(input)?;
    Ok(s.to_string())
}
pub fn unquoted_string(input: &mut Input) -> ModalResult<String> {
    take_till(1.., |c: char| c.is_whitespace() || "(){}|<>;&".contains(c))
        .verify(|string: &str| !string.starts_with('#'))
        .map(str::to_string)
        .parse_next(input)
}
