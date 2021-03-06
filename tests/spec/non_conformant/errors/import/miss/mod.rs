//! Tests auto-converted from "sass-spec/spec/non_conformant/errors/import/miss"
#[allow(unused)]
use super::rsass;

// From "sass-spec/spec/non_conformant/errors/import/miss/control-else.hrx"

// Ignoring "control_else", error tests are not supported yet.

// From "sass-spec/spec/non_conformant/errors/import/miss/control-if.hrx"

// Ignoring "control_if", error tests are not supported yet.

mod test_loop;

mod mixin;

// From "sass-spec/spec/non_conformant/errors/import/miss/simple.hrx"
#[test]
fn simple() {
    assert_eq!(
        rsass(
            "@import \"hey1.css\", \"cookie.css\", url(\"hey2.css\"), \"fudge.css\";\
            \n\
            \n$foo:\"goodbye\";\
            \ndiv[name=\"hello\"] {\
            \n  color: blue;\
            \n}\
            \n\
            \n@import \"bludge.css\";"
        )
        .unwrap(),
        "@import url(hey1.css);\
        \n@import url(cookie.css);\
        \n@import url(\"hey2.css\");\
        \n@import url(fudge.css);\
        \n@import url(bludge.css);\
        \ndiv[name=\"hello\"] {\
        \n  color: blue;\
        \n}\
        \n"
    );
}
