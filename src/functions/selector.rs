use super::SassFunction;
use crate::css::Value;
use crate::error::Error;
use crate::parser::selectors::{selector, selectors};
use crate::parser::Span;
use crate::selectors::{Selector, Selectors};
use crate::value::Quotes;
use std::collections::BTreeMap;

pub fn register(f: &mut BTreeMap<&'static str, SassFunction>) {
    def_va!(f, selector_nest(selectors), |s| match s.get("selectors")? {
        Value::List(v, _, _) => Ok(Value::Literal(
            format!(
                "{}",
                v.into_iter()
                    .map(parse_selectors)
                    .try_fold(Selectors::root(), |b, e| e
                        .map(|e| e.inside(&b)))?
            ),
            Quotes::None,
        )),
        v => Ok(Value::Literal(
            format!("{}", parse_selectors(v)?),
            Quotes::None,
        )),
    });
    def_va!(
        f,
        selector_append(selectors),
        |s| match s.get("selectors")? {
            Value::List(v, _, _) => Ok(Value::Literal(
                format!(
                    "{}",
                    v.into_iter().map(parse_selectors).try_fold(
                        Selectors::root(),
                        |base, ext| ext.and_then(|ext| Ok(Selectors::new(
                            base.s
                                .into_iter()
                                .flat_map(|b| {
                                    ext.s.iter().map(move |e| {
                                        parse_selector(&format!("{}{}", b, e))
                                    })
                                })
                                .collect::<Result<_, _>>()?
                        ))),
                    )?,
                ),
                Quotes::None,
            )),
            v => Ok(Value::Literal(
                format!("{}", parse_selectors(v)?),
                Quotes::None,
            )),
        }
    );
    def!(f, selector_parse(selector), |s| Ok(parse_selectors(
        s.get("selector")?
    )?
    .to_value()));
}

fn parse_selectors(v: Value) -> Result<Selectors, Error> {
    let s = format!("{}", v.unquote().format(Default::default()));
    if s.is_empty() {
        Ok(Selectors::root())
    } else {
        let (rest, result) = selectors(Span::new(s.as_bytes()))?;
        let rest = rest.fragment();
        if !rest.is_empty() && rest != b"," {
            Err(Error::S(format!(
                "Unexpected remains in parse_selectors: {:?}",
                rest
            )))
        } else {
            Ok(result)
        }
    }
}

fn parse_selector(s: &str) -> Result<Selector, Error> {
    let (rest, result) = selector(Span::new(s.as_bytes()))?;
    let rest = rest.fragment();
    if !rest.is_empty() {
        Err(Error::S(format!(
            "Unexpected remains in parse_selector: {:?}",
            rest
        )))
    } else {
        Ok(result)
    }
}
