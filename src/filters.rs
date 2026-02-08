#![expect(clippy::inline_always)]

use std::fmt::Display;

use askama::Values;
use pulldown_cmark::html;

#[askama::filter_fn]
pub fn markdown<T: Display>(s: T, _: &dyn Values) -> askama::Result<String> {
    let binding = s.to_string();
    let parser = pulldown_cmark::Parser::new(&binding);

    let mut output = String::new();

    html::push_html(&mut output, parser);

    Ok(output)
}
