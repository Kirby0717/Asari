use super::*;

pub fn unicode_number(input: &mut Input) -> ModalResult<char> {
    take_until(0.., '}')
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(UnicodeEscapeError::NoEndBrace)
        })
        .try_map_with_span(|input| {
            let code = u32::from_str_radix(input, 16)
                .map_err(ParseErrorKind::ParseHexError)?;
            char::from_u32(code).ok_or(ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::InvalidUnicode,
            ))
        })
        .parse_next(input)
}
pub fn unicode_escape_char(input: &mut Input) -> ModalResult<char> {
    let _ = 'u'.parse_next(input)?;
    let _ = '{'
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(
                UnicodeEscapeError::NoBeginBrace,
            )
        })
        .cut()
        .parse_next(input)?;
    let c = unicode_number.cut().parse_next(input)?;
    let _ = '}'
        .map_err_with_span(|()| {
            ParseErrorKind::InvalidUnicodeEscape(UnicodeEscapeError::NoEndBrace)
        })
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
            'u' => unicode_escape_char,
            '\\' => any.value('\\'),
            '\"' => any.value('\"'),
            '\'' => any.value('\''),
            '0' => any.value('\0'),
            c => any.try_map_with_span(|_| {
                Err(ParseErrorKind::UnrecognizedEscape(c))
            }).cut(),
        ),
    )
    .parse_next(input)
}

pub fn quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\'';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER
            .map_err_with_span(|()| ParseErrorKind::NoEndQuotation)
            .cut(),
    )
    .parse_next(input)
}
pub fn double_quoted_string(input: &mut Input) -> ModalResult<String> {
    const DELIMITER: char = '\"';
    delimited(
        DELIMITER,
        repeat(0.., alt((escape_char, any.verify(|c| *c != DELIMITER)))),
        DELIMITER
            .map_err_with_span(|()| ParseErrorKind::NoEndDoubleQuotation)
            .cut(),
    )
    .parse_next(input)
}
pub fn raw_string(input: &mut Input) -> ModalResult<String> {
    let _ = 'r'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
pub fn path_string(input: &mut Input) -> ModalResult<String> {
    let _ = 'p'.parse_next(input)?;
    let sharp = take_while(0.., '#').parse_next(input)?;
    let _ = '"'.parse_next(input)?;
    let delimiter = '"'.to_string() + sharp;
    let raw = take_until(0.., delimiter.as_str()).parse_next(input)?;
    let _ = delimiter.as_str().parse_next(input)?;
    Ok(raw.to_string())
}
pub fn unquoted_string(input: &mut Input) -> ModalResult<String> {
    take_till(1.., |c: char| c.is_whitespace() || "(){}|<>;&".contains(c))
        .map(str::to_string)
        .parse_next(input)
}
