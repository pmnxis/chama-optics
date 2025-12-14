//
// Copyright (c) 2016 KAMADA Ken'ichi.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.
//

// This source code is modified from EXIF-rs for some dirty float dataform.

// Taken from exif-rs
fn d_sub_comma<I, T>(w: &mut dyn std::fmt::Write, itit: I) -> std::fmt::Result
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Display,
{
    let mut first = true;
    for x in itit {
        match first {
            true => write!(w, "{x}"),
            false => write!(w, ", {x}"),
        }?;
        first = false;
    }
    Ok(())
}

fn d_sub_hex(w: &mut dyn std::fmt::Write, bytes: &[u8]) -> std::fmt::Result {
    w.write_str("0x")?;
    for x in bytes {
        write!(w, "{x:02x}")?;
    }
    Ok(())
}

struct AsciiDisplay<'a>(&'a [u8]);
impl<'a> std::fmt::Display for AsciiDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.0 {
            let c = if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            };
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

// Some image has dirty float on this field.
// Ex : 1.399999976158142 ->
pub(crate) fn d_decimal(
    w: &mut dyn std::fmt::Write,
    value: &exif::Value,
    round_up: i32,
) -> std::fmt::Result {
    let factor = 10f64.powi(round_up);

    match *value {
        exif::Value::Rational(ref v) => {
            d_sub_comma(w, v.iter().map(|x| (x.to_f64() * factor).round() / factor))
        }
        exif::Value::SRational(ref v) => {
            d_sub_comma(w, v.iter().map(|x| (x.to_f64() * factor).round() / factor))
        }
        _ => d_default(w, value),
    }
}

// Some image has dirty float on this field.
// Ex : 1/50.00000875872
pub(crate) fn d_exptime(w: &mut dyn std::fmt::Write, value: &exif::Value) -> std::fmt::Result {
    if let Some(et) = match value {
        exif::Value::Rational(x) => x.first(),
        _ => None,
    } {
        if et.num >= et.denom {
            let sec = et.num as f64 / et.denom as f64;
            let rounded = (sec * 100.0).round() / 100.0;

            return if (rounded.fract()).abs() < 0.001 {
                write!(w, "{}", rounded.round() as i64)
            } else {
                write!(w, "{rounded:.2}")
            };
        } else if et.num != 0 {
            let denom = et.denom as f64 / et.num as f64;
            let rounded = (denom * 100.0).round() / 100.0;

            return if (rounded.fract()).abs() < 0.001 {
                write!(w, "1/{}", rounded.round() as i64)
            } else {
                write!(w, "1/{rounded:.2}")
            };
        }
    }
    d_default(w, value)
}

pub(crate) fn d_default(w: &mut dyn std::fmt::Write, value: &exif::Value) -> std::fmt::Result {
    use exif::Value;
    match *value {
        Value::Byte(ref v) => d_sub_comma(w, v),
        Value::Ascii(ref v) => d_sub_comma(w, v.iter().map(|x| AsciiDisplay(x))),
        Value::Short(ref v) => d_sub_comma(w, v),
        Value::Long(ref v) => d_sub_comma(w, v),
        Value::Rational(ref v) => d_sub_comma(w, v),
        Value::SByte(ref v) => d_sub_comma(w, v),
        Value::Undefined(ref v, _) => d_sub_hex(w, v),
        Value::SShort(ref v) => d_sub_comma(w, v),
        Value::SLong(ref v) => d_sub_comma(w, v),
        Value::SRational(ref v) => d_sub_comma(w, v),
        Value::Float(ref v) => d_sub_comma(w, v),
        Value::Double(ref v) => d_sub_comma(w, v),
        Value::Unknown(t, c, o) => write!(w, "unknown value (type={t}, count={c}, offset={o:#x})"),
        #[allow(unreachable_patterns)]
        _ => {
            unimplemented!()
        }
    }
}
