use super::*;
use regress::{Match, Regex};
use crate::errors::{range_error, syntax_error};

pub(super) fn find(expression: &JsRegExp, input: &str, start: usize, sticky: bool) -> JsResult<Option<Match>> {
    if start > input.len() { return Ok(None); }
    if !input.is_char_boundary(start) {
        return Err(range_error("RegExp index lies inside a native UTF-8 character"));
    }
    let compiled = &expression.state.compiled;
    let regex = compiled.native_regex.get_or_init(|| {
        Regex::from_unicode(
            super::super::pattern_code_points(&compiled.source, true).into_iter(),
            compiled.parsed_flags.engine_flags(),
        ).map_err(|error| syntax_error(format!("invalid native regular expression: {error}")))
    }).as_ref().map_err(Clone::clone)?;
    regex.try_find_from(input, start, super::super::execution_budget(input.len()))
        .map(|found| found.filter(|matched| !sticky || matched.start() == start))
        .map_err(|error| range_error(error.to_string()))
}

pub(super) fn execute(expression: &JsRegExp, input: &str) -> JsResult<Option<Match>> {
    let stateful = expression.global() || expression.sticky();
    let start = if stateful { super::super::to_length(expression.last_index()) } else { 0 };
    let found = find(expression, input, start, expression.sticky())?;
    if stateful { expression.set_last_index(found.as_ref().map_or(0.0, |found| found.end() as f64)); }
    Ok(found)
}

pub(super) fn advance(input: &str, index: usize) -> usize {
    index + input.get(index..).and_then(|tail| tail.chars().next()).map_or(1, char::len_utf8)
}

pub(super) fn build(expression: &JsRegExp, input: &str, matched: Match) -> RegExpExecArray {
    let values = super::super::array_from_optional(matched.groups().map(|range| range.map(|span| input[span].to_owned())).collect());
    let groups: BTreeMap<_, _> = matched.named_groups()
        .map(|(name, range)| (name.to_owned(), range.map(|span| input[span].to_owned())))
        .collect();
    let indices = expression.has_indices().then(|| {
        let values = super::super::array_from_optional(matched.groups()
            .map(|range| range.map(|span| (span.start as f64, span.end as f64))).collect());
        let named: BTreeMap<_, _> = matched.named_groups()
            .map(|(name, range)| (name.to_owned(), range.map(|span| (span.start as f64, span.end as f64))))
            .collect();
        RegExpIndices { values, groups: (!named.is_empty()).then(|| RegExpNamedIndices { values: Rc::new(RefCell::new(named)) }) }
    });
    RegExpExecArray {
        values,
        index: matched.start() as f64,
        input: input.to_owned(),
        groups: (!groups.is_empty()).then(|| RegExpNamedGroups { values: Rc::new(RefCell::new(groups)) }),
        indices,
    }
}

pub(super) fn collect(expression: &JsRegExp, input: &str) -> JsResult<Vec<Match>> {
    let mut matches = Vec::new();
    if expression.global() { expression.set_last_index(0.0); }
    while let Some(found) = execute(expression, input)? {
        let end = found.end();
        let empty = found.start() == end;
        matches.push(found);
        if !expression.global() { break; }
        if empty { expression.set_last_index(advance(input, end) as f64); }
    }
    Ok(matches)
}

pub(super) fn append_substitution(output: &mut String, input: &str, matched: &Match, replacement: &str) {
    let mut cursor = 0;
    let bytes = replacement.as_bytes();
    while cursor < bytes.len() {
        let Some(relative) = replacement[cursor..].find('$') else {
            output.push_str(&replacement[cursor..]);
            break;
        };
        let start = cursor + relative;
        output.push_str(&replacement[cursor..start]);
        cursor = start + 1;
        match bytes.get(cursor).copied() {
            Some(b'$') => { output.push('$'); cursor += 1; }
            Some(b'&') => { output.push_str(&input[matched.range.clone()]); cursor += 1; }
            Some(b'`') => { output.push_str(&input[..matched.start()]); cursor += 1; }
            Some(b'\'') => { output.push_str(&input[matched.end()..]); cursor += 1; }
            Some(b'<') if matched.named_groups().next().is_some() => {
                if let Some(end) = replacement[cursor + 1..].find('>') {
                    let name = &replacement[cursor + 1..cursor + 1 + end];
                    if let Some((_, Some(range))) = matched.named_groups().find(|(candidate, _)| *candidate == name) {
                        output.push_str(&input[range]);
                    }
                    cursor += end + 2;
                } else { output.push('$'); }
            }
            Some(digit @ b'0'..=b'9') => {
                let mut number = usize::from(digit - b'0');
                let mut count = 1;
                if let Some(next @ b'0'..=b'9') = bytes.get(cursor + 1).copied() {
                    let pair = number * 10 + usize::from(next - b'0');
                    if pair > 0 && pair <= matched.captures.len() { number = pair; count = 2; }
                }
                if number > 0 && number <= matched.captures.len() {
                    if let Some(range) = matched.group(number) { output.push_str(&input[range]); }
                    cursor += count;
                } else { output.push('$'); }
            }
            _ => output.push('$'),
        }
    }
}

pub(super) fn replacement_arguments(input: &str, matched: &Match) -> JsArray<JsValue> {
    let mut arguments: Vec<_> = matched.groups().map(|range| range.map_or(JsValue::Undefined,
        |span| JsValue::String((&input[span]).to_owned()))).collect();
    arguments.push(JsValue::Number(matched.start() as f64));
    arguments.push(JsValue::String((input).to_owned()));
    if matched.named_groups().next().is_some() {
        let mut groups = crate::JsObject::new();
        for (name, range) in matched.named_groups() {
            groups.set(name, range.map_or(JsValue::Undefined, |span| JsValue::String((&input[span]).to_owned())));
        }
        arguments.push(JsValue::object(groups));
    }
    JsArray::from_dense(arguments)
}
