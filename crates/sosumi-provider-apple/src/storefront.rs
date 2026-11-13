//! Storefront identifiers.
//!
//! Apple's Android library reports an account's storefront as a numeric id with a
//! platform and content-class suffix, for example `143467-2,31`. The web API wants
//! a two-letter region code, so the id has to be *translated*, not trimmed:
//! `143467` is India. A fallback that guessed would send every non-US account's
//! requests to the wrong storefront, which surfaces as an empty catalog rather
//! than as an error.
//!
//! The table is a fixed list because there is no arithmetic relationship between
//! the numbers and the regions they name. It is the mapping Apple's web client
//! uses.

/// Translates a storefront identifier into the two-letter region code.
///
/// Accepts either form, since the file on disk holds the numeric id but a hand
/// written configuration may already carry the code. Returns `None` for a value
/// that is neither, leaving the fallback to the caller rather than choosing a
/// region on the user's behalf.
#[must_use]
pub fn region_for(storefront_id: &str) -> Option<String> {
    let raw = storefront_id.trim();
    if raw.is_empty() {
        return None;
    }

    // Already a region code. Kept as-is rather than validated against the table,
    // which is keyed by number; the value came from Apple either way.
    if raw.len() == 2 && raw.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(raw.to_ascii_lowercase());
    }

    // Numeric form: drop the `-2,31` suffix, then translate through the table.
    let numeric = raw.split(['-', ',']).next().unwrap_or_default();
    numeric_region(numeric).map(str::to_owned)
}

/// The region for a bare numeric storefront id, e.g. `143467` is India.
///
/// Split out so the table can stay a `&'static str` match — one arm per region,
/// no allocation — while [`region_for`] returns an owned code for the caller.
fn numeric_region(storefront_id: &str) -> Option<&'static str> {
    match storefront_id {
        "143441" => Some("us"),
        "143442" => Some("fr"),
        "143443" => Some("de"),
        "143444" => Some("gb"),
        "143445" => Some("at"),
        "143446" => Some("be"),
        "143447" => Some("fi"),
        "143448" => Some("gr"),
        "143449" => Some("ie"),
        "143450" => Some("it"),
        "143451" => Some("lu"),
        "143452" => Some("nl"),
        "143453" => Some("pt"),
        "143454" => Some("es"),
        "143455" => Some("ca"),
        "143456" => Some("se"),
        "143457" => Some("no"),
        "143458" => Some("dk"),
        "143459" => Some("ch"),
        "143460" => Some("au"),
        "143461" => Some("nz"),
        "143462" => Some("jp"),
        "143463" => Some("hk"),
        "143464" => Some("sg"),
        "143465" => Some("cn"),
        "143466" => Some("kr"),
        "143467" => Some("in"),
        "143468" => Some("mx"),
        "143469" => Some("ru"),
        "143470" => Some("tw"),
        "143471" => Some("vn"),
        "143472" => Some("za"),
        "143473" => Some("my"),
        "143474" => Some("ph"),
        "143475" => Some("th"),
        "143476" => Some("id"),
        "143477" => Some("pk"),
        "143478" => Some("pl"),
        "143479" => Some("sa"),
        "143480" => Some("tr"),
        "143481" => Some("ae"),
        "143482" => Some("hu"),
        "143483" => Some("cl"),
        "143484" => Some("np"),
        "143485" => Some("pa"),
        "143486" => Some("lk"),
        "143487" => Some("ro"),
        "143488" => Some("mv"),
        "143489" => Some("cz"),
        "143490" => Some("bd"),
        "143491" => Some("il"),
        "143492" => Some("ua"),
        "143493" => Some("kw"),
        "143494" => Some("hr"),
        "143495" => Some("cr"),
        "143496" => Some("sk"),
        "143497" => Some("lb"),
        "143498" => Some("qa"),
        "143499" => Some("si"),
        "143500" => Some("rs"),
        "143501" => Some("co"),
        "143502" => Some("ve"),
        "143503" => Some("br"),
        "143504" => Some("gt"),
        "143505" => Some("ar"),
        "143506" => Some("sv"),
        "143507" => Some("pe"),
        "143508" => Some("do"),
        "143509" => Some("ec"),
        "143510" => Some("hn"),
        "143511" => Some("jm"),
        "143512" => Some("ni"),
        "143513" => Some("py"),
        "143514" => Some("uy"),
        "143515" => Some("mo"),
        "143516" => Some("eg"),
        "143517" => Some("kz"),
        "143518" => Some("ee"),
        "143519" => Some("lv"),
        "143520" => Some("lt"),
        "143521" => Some("mt"),
        "143522" => Some("li"),
        "143523" => Some("md"),
        "143524" => Some("am"),
        "143525" => Some("bw"),
        "143526" => Some("bg"),
        "143527" => Some("ci"),
        "143528" => Some("jo"),
        "143529" => Some("ke"),
        "143530" => Some("mk"),
        "143531" => Some("mg"),
        "143532" => Some("ml"),
        "143533" => Some("mu"),
        "143534" => Some("ne"),
        "143535" => Some("sn"),
        "143536" => Some("tn"),
        "143537" => Some("ug"),
        "143538" => Some("ai"),
        "143539" => Some("bs"),
        "143540" => Some("ag"),
        "143541" => Some("bb"),
        "143542" => Some("bm"),
        "143543" => Some("vg"),
        "143544" => Some("ky"),
        "143545" => Some("dm"),
        "143546" => Some("gd"),
        "143547" => Some("ms"),
        "143548" => Some("kn"),
        "143549" => Some("lc"),
        "143550" => Some("vc"),
        "143551" => Some("tt"),
        "143552" => Some("tc"),
        "143553" => Some("gy"),
        "143554" => Some("sr"),
        "143555" => Some("bz"),
        "143556" => Some("bo"),
        "143557" => Some("cy"),
        "143558" => Some("is"),
        "143559" => Some("bh"),
        "143560" => Some("bn"),
        "143561" => Some("ng"),
        "143562" => Some("om"),
        "143563" => Some("dz"),
        "143564" => Some("ao"),
        "143565" => Some("by"),
        "143566" => Some("uz"),
        "143567" => Some("ly"),
        "143568" => Some("az"),
        "143569" => Some("et"),
        "143570" => Some("mm"),
        "143571" => Some("ye"),
        "143572" => Some("tz"),
        "143573" => Some("gh"),
        "143574" => Some("cm"),
        "143575" => Some("al"),
        "143576" => Some("bj"),
        "143577" => Some("bt"),
        "143578" => Some("bf"),
        "143579" => Some("kh"),
        "143580" => Some("cv"),
        "143581" => Some("td"),
        "143582" => Some("cg"),
        "143583" => Some("fj"),
        "143584" => Some("gm"),
        "143585" => Some("gw"),
        "143586" => Some("kg"),
        "143587" => Some("la"),
        "143588" => Some("lr"),
        "143589" => Some("mw"),
        "143590" => Some("mr"),
        "143591" => Some("fm"),
        "143592" => Some("mn"),
        "143593" => Some("mz"),
        "143594" => Some("na"),
        "143595" => Some("pw"),
        "143596" => Some("ps"),
        "143597" => Some("pg"),
        "143598" => Some("st"),
        "143599" => Some("sc"),
        "143600" => Some("sl"),
        "143601" => Some("sb"),
        "143602" => Some("sz"),
        "143603" => Some("tj"),
        "143604" => Some("tm"),
        "143605" => Some("zw"),
        "143606" => Some("nr"),
        "143607" => Some("ws"),
        "143608" => Some("to"),
        "143609" => Some("vu"),
        "143610" => Some("af"),
        "143611" => Some("ad"),
        "143612" => Some("ba"),
        "143613" => Some("cd"),
        "143614" => Some("ga"),
        "143615" => Some("ge"),
        "143616" => Some("gn"),
        "143617" => Some("iq"),
        "143618" => Some("mc"),
        "143619" => Some("me"),
        "143620" => Some("ma"),
        "143621" => Some("rw"),
        "143622" => Some("zm"),
        "143623" => Some("cf"),
        "143624" => Some("xk"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_numeric_id_becomes_its_region() {
        // The real value Apple's Android library writes. India, not the United
        // States: this is the case a trimming implementation gets wrong.
        assert_eq!(region_for("143467-2,31").as_deref(), Some("in"));
        assert_eq!(region_for("143441-1,29").as_deref(), Some("us"));
        assert_eq!(region_for("143462-2,31").as_deref(), Some("jp"));
        assert_eq!(region_for("143503-2,31").as_deref(), Some("br"));
    }

    #[test]
    fn a_region_code_passes_through() {
        assert_eq!(region_for("gb").as_deref(), Some("gb"));
        assert_eq!(region_for("GB").as_deref(), Some("gb"));
        assert_eq!(region_for(" jp ").as_deref(), Some("jp"));
    }

    #[test]
    fn an_unknown_value_has_no_region() {
        // `None` rather than a guess, so the caller chooses the fallback and the
        // choice is visible at the call site.
        assert_eq!(region_for(""), None);
        assert_eq!(region_for("999999-1,29"), None);
        assert_eq!(region_for("not a storefront"), None);
    }
}
