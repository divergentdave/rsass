//! Color names from <https://www.w3.org/TR/css3-color/>
#![allow(clippy::unreadable_literal)]

use crate::output::{Format, Formatted};
use crate::value::Number;
use lazy_static::lazy_static;
use num_rational::Rational;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::ops::{Add, Div, Sub};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Rgba {
    pub red: Rational,
    pub green: Rational,
    pub blue: Rational,
    pub alpha: Rational,
}

impl Rgba {
    pub fn new(r: Rational, g: Rational, b: Rational, a: Rational) -> Self {
        let ff = Rational::new(255, 1);
        let one = Rational::one();
        Rgba {
            red: cap(r, &ff),
            green: cap(g, &ff),
            blue: cap(b, &ff),
            alpha: cap(a, &one),
        }
    }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Rgba {
            red: Rational::from_integer(r as isize),
            green: Rational::from_integer(g as isize),
            blue: Rational::from_integer(b as isize),
            alpha: Rational::one(),
        }
    }
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Rgba {
            red: Rational::from_integer(r as isize),
            green: Rational::from_integer(g as isize),
            blue: Rational::from_integer(b as isize),
            alpha: Rational::from_integer(a as isize) / 255,
        }
    }
    pub fn from_hsla(
        hue: Rational,
        sat: Rational,
        lig: Rational,
        a: Rational,
    ) -> Self {
        let sat = cap(sat, &Rational::one());
        let lig = cap(lig, &Rational::one());
        if sat.is_zero() {
            let gray = lig * 255;
            Rgba::new(gray, gray, gray, a)
        } else {
            fn hue2rgb(p: Rational, q: Rational, t: Rational) -> Rational {
                let t = (t - t.floor()) * 6;
                match t.to_integer() {
                    0 => p + (q - p) * t,
                    1 | 2 => q,
                    3 => p + (p - q) * (t - 4),
                    _ => p,
                }
            }
            let q = if lig < Rational::new(1, 2) {
                lig * (sat + 1)
            } else {
                lig + sat - lig * sat
            };
            let p = lig * 2 - q;

            Rgba::new(
                hue2rgb(p, q, hue + Rational::new(1, 3)) * 255,
                hue2rgb(p, q, hue) * 255,
                hue2rgb(p, q, hue - Rational::new(1, 3)) * 255,
                a,
            )
        }
    }
    pub fn name(&self) -> Option<&'static str> {
        if self.alpha >= Rational::one() {
            let (r, g, b, _a) = self.to_bytes();
            let c = u32::from_be_bytes([0, r, g, b]);
            LOOKUP.v2n.get(&c).copied()
        } else {
            None
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_lowercase();
        let name: &str = &name;
        if name == "transparent" {
            return Some(Self::from_rgba(0, 0, 0, 0));
        }
        LOOKUP.n2v.get(name).map(|n| {
            let [_, r, g, b] = n.to_be_bytes();
            Self::from_rgb(r, g, b)
        })
    }

    pub fn all_zero(&self) -> bool {
        self.alpha.is_zero()
            && self.red.is_zero()
            && self.green.is_zero()
            && self.blue.is_zero()
    }
    pub fn to_bytes(&self) -> (u8, u8, u8, u8) {
        fn byte(v: Rational) -> u8 {
            v.round().to_integer() as u8
        }
        let a = self.alpha * 255;
        (byte(self.red), byte(self.green), byte(self.blue), byte(a))
    }
    /// Convert rgb (0 .. 255) to hue (degrees) / sat (0 .. 1) / lighness (0 .. 1)
    pub fn to_hsla(&self) -> (Rational, Rational, Rational, Rational) {
        let (red, green, blue) =
            (self.red / 255, self.green / 255, self.blue / 255);
        let (max, min, largest) = max_min_largest(red, green, blue);

        if max == min {
            (Rational::zero(), Rational::zero(), max, self.alpha)
        } else {
            let d = max - min;
            let h = match largest {
                0 => (green - blue) / d + if green < blue { 6 } else { 0 },
                1 => (blue - red) / d + 2,
                _ => (red - green) / d + 4,
            } * 360
                / 6;
            let mm = max + min;
            let s = d / if mm > Rational::one() { -mm + 2 } else { mm };
            (h, s, mm / 2, self.alpha)
        }
    }
    pub fn format(&self, format: Format) -> Formatted<Rgba> {
        Formatted {
            value: self,
            format,
        }
    }
}

fn cap(n: Rational, ff: &Rational) -> Rational {
    if n > *ff {
        *ff
    } else if n.is_negative() {
        Rational::zero()
    } else {
        n
    }
}

impl Add<Rational> for Rgba {
    type Output = Rgba;

    fn add(self, rhs: Rational) -> Rgba {
        Rgba::new(
            self.red + rhs,
            self.green + rhs,
            self.blue + rhs,
            self.alpha,
        )
    }
}

impl Add<Rgba> for Rgba {
    type Output = Rgba;

    fn add(self, rhs: Rgba) -> Rgba {
        Rgba::new(
            self.red + rhs.red,
            self.green + rhs.green,
            self.blue + rhs.blue,
            // TODO Sum or average the alpha?
            self.alpha + rhs.alpha,
        )
    }
}

impl<'a> Div<Rational> for &'a Rgba {
    type Output = Rgba;

    fn div(self, rhs: Rational) -> Rgba {
        Rgba::new(
            self.red / rhs,
            self.green / rhs,
            self.blue / rhs,
            self.alpha,
        )
    }
}

impl<'a> Sub<Rational> for &'a Rgba {
    type Output = Rgba;

    fn sub(self, rhs: Rational) -> Rgba {
        Rgba::new(
            self.red - rhs,
            self.green - rhs,
            self.blue - rhs,
            self.alpha,
        )
    }
}

impl<'a> Sub<&'a Rgba> for &'a Rgba {
    type Output = Rgba;

    fn sub(self, rhs: &'a Rgba) -> Rgba {
        Rgba::new(
            self.red - rhs.red,
            self.green - rhs.green,
            self.blue - rhs.blue,
            (self.alpha + rhs.alpha) / 2,
        )
    }
}

impl<'a> Display for Formatted<'a, Rgba> {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        // The byte-version of alpha is not used here.
        let (r, g, b, _a) = self.value.to_bytes();
        let a = self.value.alpha;
        if a >= Rational::one() {
            // E.g. #ff00cc can be written #f0c in css.
            // 0xff / 0x11 = 0xf.
            let short = r % 0x11 == 0 && g % 0x11 == 0 && b % 0x11 == 0;
            let hex_len = if short { 4 } else { 7 };
            if let Some(name) = self.value.name() {
                if !(self.format.is_compressed() && name.len() > hex_len) {
                    return name.fmt(out);
                }
            }
            if self.format.is_compressed() && short {
                write!(out, "#{:x}{:x}{:x}", r / 0x11, g / 0x11, b / 0x11)
            } else {
                write!(out, "#{:02x}{:02x}{:02x}", r, g, b)
            }
        } else if self.format.is_compressed() && self.value.all_zero() {
            write!(out, "transparent")
        } else if self.format.is_compressed() {
            // Note: libsass does not use the format for the alpha like this.
            let a = Number::from(a);
            write!(out, "rgba({},{},{},{})", r, g, b, a.format(self.format))
        } else {
            let a = Number::from(a);
            write!(
                out,
                "rgba({}, {}, {}, {})",
                r,
                g,
                b,
                a.format(self.format)
            )
        }
    }
}

// Find which of three numbers are largest and smallest
fn max_min_largest(
    a: Rational,
    b: Rational,
    c: Rational,
) -> (Rational, Rational, u32) {
    let v = [(a, 0), (b, 1), (c, 2)];
    let max = v.iter().max().unwrap();
    let min = v.iter().min().unwrap();
    (max.0, min.0, max.1)
}

#[test]
fn get_black() {
    assert_eq!(Rgba::from_rgb(0, 0, 0).name(), Some("black"))
}

#[test]
fn get_none() {
    assert_eq!(Rgba::from_rgb(0, 1, 2).name(), None)
}

#[test]
fn get_red_by_name() {
    assert_eq!(Some(Rgba::from_rgb(255, 0, 0)), Rgba::from_name("red"));
}

#[test]
fn get_none_by_name() {
    assert_eq!(None, Rgba::from_name("xyzzy"));
}

struct Lookup {
    n2v: BTreeMap<&'static str, u32>,
    v2n: BTreeMap<u32, &'static str>,
}

impl Lookup {
    fn from_slice(data: &[(&'static str, u32)]) -> Self {
        let mut n2v: BTreeMap<&'static str, u32> = BTreeMap::new();
        let mut v2n: BTreeMap<u32, &'static str> = BTreeMap::new();
        for &(n, v) in data {
            n2v.insert(n, v);
            v2n.entry(v).or_insert(n);
        }
        Lookup { n2v, v2n }
    }
}

lazy_static! {
    static ref LOOKUP: Lookup = Lookup::from_slice(&[
        ("aliceblue", 0xf0f8ff_u32),
        ("antiquewhite", 0xfaebd7),
        ("aqua", 0x00ffff),
        ("aquamarine", 0x7fffd4),
        ("azure", 0xf0ffff),
        ("beige", 0xf5f5dc),
        ("bisque", 0xffe4c4),
        ("black", 0x000000),
        ("blanchedalmond", 0xffebcd),
        ("blue", 0x0000ff),
        ("blueviolet", 0x8a2be2),
        ("brown", 0xa52a2a),
        ("burlywood", 0xdeb887),
        ("cadetblue", 0x5f9ea0),
        ("chartreuse", 0x7fff00),
        ("chocolate", 0xd2691e),
        ("coral", 0xff7f50),
        ("cornflowerblue", 0x6495ed),
        ("cornsilk", 0xfff8dc),
        ("crimson", 0xdc143c),
        ("cyan", 0x00ffff),
        ("darkblue", 0x00008b),
        ("darkcyan", 0x008b8b),
        ("darkgoldenrod", 0xb8860b),
        ("darkgray", 0xa9a9a9),
        ("darkgreen", 0x006400),
        ("darkgrey", 0xa9a9a9),
        ("darkkhaki", 0xbdb76b),
        ("darkmagenta", 0x8b008b),
        ("darkolivegreen", 0x556b2f),
        ("darkorange", 0xff8c00),
        ("darkorchid", 0x9932cc),
        ("darkred", 0x8b0000),
        ("darksalmon", 0xe9967a),
        ("darkseagreen", 0x8fbc8f),
        ("darkslateblue", 0x483d8b),
        ("darkslategray", 0x2f4f4f),
        ("darkslategrey", 0x2f4f4f),
        ("darkturquoise", 0x00ced1),
        ("darkviolet", 0x9400d3),
        ("deeppink", 0xff1493),
        ("deepskyblue", 0x00bfff),
        ("dimgrey", 0x696969),
        ("dimgray", 0x696969),
        ("dodgerblue", 0x1e90ff),
        ("firebrick", 0xb22222),
        ("floralwhite", 0xfffaf0),
        ("forestgreen", 0x228b22),
        ("fuchsia", 0xff00ff),
        ("gainsboro", 0xdcdcdc),
        ("ghostwhite", 0xf8f8ff),
        ("gold", 0xffd700),
        ("goldenrod", 0xdaa520),
        ("gray", 0x808080),
        ("grey", 0x808080),
        ("green", 0x008000),
        ("greenyellow", 0xadff2f),
        ("honeydew", 0xf0fff0),
        ("hotpink", 0xff69b4),
        ("indianred", 0xcd5c5c),
        ("indigo", 0x4b0082),
        ("ivory", 0xfffff0),
        ("khaki", 0xf0e68c),
        ("lavender", 0xe6e6fa),
        ("lavenderblush", 0xfff0f5),
        ("lawngreen", 0x7cfc00),
        ("lemonchiffon", 0xfffacd),
        ("lightblue", 0xadd8e6),
        ("lightcoral", 0xf08080),
        ("lightcyan", 0xe0ffff),
        ("lightgoldenrodyellow", 0xfafad2),
        ("lightgray", 0xd3d3d3),
        ("lightgreen", 0x90ee90),
        ("lightgrey", 0xd3d3d3),
        ("lightpink", 0xffb6c1),
        ("lightsalmon", 0xffa07a),
        ("lightseagreen", 0x20b2aa),
        ("lightskyblue", 0x87cefa),
        ("lightslategray", 0x778899),
        ("lightslategrey", 0x778899),
        ("lightsteelblue", 0xb0c4de),
        ("lightyellow", 0xffffe0),
        ("lime", 0x00ff00),
        ("limegreen", 0x32cd32),
        ("linen", 0xfaf0e6),
        ("magenta", 0xff00ff),
        ("maroon", 0x800000),
        ("mediumaquamarine", 0x66cdaa),
        ("mediumblue", 0x0000cd),
        ("mediumorchid", 0xba55d3),
        ("mediumpurple", 0x9370db),
        ("mediumseagreen", 0x3cb371),
        ("mediumslateblue", 0x7b68ee),
        ("mediumspringgreen", 0x00fa9a),
        ("mediumturquoise", 0x48d1cc),
        ("mediumvioletred", 0xc71585),
        ("midnightblue", 0x191970),
        ("mintcream", 0xf5fffa),
        ("mistyrose", 0xffe4e1),
        ("moccasin", 0xffe4b5),
        ("navajowhite", 0xffdead),
        ("navy", 0x000080),
        ("oldlace", 0xfdf5e6),
        ("olive", 0x808000),
        ("olivedrab", 0x6b8e23),
        ("orange", 0xffa500),
        ("orangered", 0xff4500),
        ("orchid", 0xda70d6),
        ("palegoldenrod", 0xeee8aa),
        ("palegreen", 0x98fb98),
        ("paleturquoise", 0xafeeee),
        ("palevioletred", 0xdb7093),
        ("papayawhip", 0xffefd5),
        ("peachpuff", 0xffdab9),
        ("peru", 0xcd853f),
        ("pink", 0xffc0cb),
        ("plum", 0xdda0dd),
        ("powderblue", 0xb0e0e6),
        ("purple", 0x800080),
        ("red", 0xff0000),
        ("rosybrown", 0xbc8f8f),
        ("royalblue", 0x4169e1),
        ("saddlebrown", 0x8b4513),
        ("salmon", 0xfa8072),
        ("sandybrown", 0xf4a460),
        ("seagreen", 0x2e8b57),
        ("seashell", 0xfff5ee),
        ("sienna", 0xa0522d),
        ("silver", 0xc0c0c0),
        ("skyblue", 0x87ceeb),
        ("slateblue", 0x6a5acd),
        ("slategray", 0x708090),
        ("slategrey", 0x708090),
        ("snow", 0xfffafa),
        ("springgreen", 0x00ff7f),
        ("steelblue", 0x4682b4),
        ("tan", 0xd2b48c),
        ("teal", 0x008080),
        ("thistle", 0xd8bfd8),
        ("tomato", 0xff6347),
        ("turquoise", 0x40e0d0),
        ("violet", 0xee82ee),
        ("wheat", 0xf5deb3),
        ("white", 0xffffff),
        ("whitesmoke", 0xf5f5f5),
        ("yellow", 0xffff00),
        ("yellowgreen", 0x9acd32),
    ]);
}
