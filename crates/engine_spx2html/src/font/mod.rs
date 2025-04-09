use ahash::AHashMap;

use ttf_parser::{
    gsub::{AlternateSubstitution, SingleSubstitution, SubstitutionSubtable},
    opentype_layout::Lookup,
    GlyphId, Tag,
};

trait VariantGlyphVisitor {
    /// Visit a variant glyph.
    fn visit_glyph(&self, dglyph: GlyphId, f: impl FnMut(GlyphId)) -> Option<()>;
}

impl VariantGlyphVisitor for AlternateSubstitution<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, mut f: impl FnMut(GlyphId)) -> Option<()> {
        self.coverage
            .get(dglyph)
            .and_then(|index| self.alternate_sets.get(index))
            .map(|ref a| a.alternates)
            .iter()
            .for_each(|set| {
                set.into_iter().for_each(|g| f(g));
            });
        Some(())
    }
}

impl VariantGlyphVisitor for SingleSubstitution<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, mut f: impl FnMut(GlyphId)) -> Option<()> {
        match *self {
            SingleSubstitution::Format1 { coverage, delta } => {
                if let Some(_) = coverage.get(dglyph) {
                    f(GlyphId((i32::from(dglyph.0) + i32::from(delta)) as u16));
                }
            }
            SingleSubstitution::Format2 {
                coverage,
                substitutes,
            } => {
                if let Some(index) = coverage.get(dglyph) {
                    f(substitutes.get(index)?);
                }
            }
        }
        Some(())
    }
}

impl VariantGlyphVisitor for ttf_parser::math::Variants<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, mut f: impl FnMut(GlyphId)) -> Option<()> {
        // Do we need to pass on whether a variant glyph is horizontal or vertical to upstream?

        if let Some(hvars) = self
            .horizontal_constructions
            .get(dglyph)
            .map(|ref gc| gc.variants)
        {
            for hvar in hvars {
                f(hvar.variant_glyph);
            }
        }

        if let Some(vvars) = self
            .vertical_constructions
            .get(dglyph)
            .map(|ref gc| gc.variants)
        {
            for vvar in vvars {
                f(vvar.variant_glyph);
            }
        }

        Some(())
    }
}

impl VariantGlyphVisitor for Lookup<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, mut f: impl FnMut(GlyphId)) -> Option<()> {
        use SubstitutionSubtable::*;

        for subtable in self.subtables.into_iter::<SubstitutionSubtable>() {
            match subtable {
                Single(t) => t.visit_glyph(dglyph, &mut f)?,
                Alternate(t) => t.visit_glyph(dglyph, &mut f)?,
                _ => {}
            }
        }
        Some(())
    }
}

pub(crate) fn load_lookup(
    reverse_gmap: &mut ReverseGlyphMap,
    lookup: &Lookup<'_>,
    dglyphs: &[(char, GlyphId)],
) {
    for (c, v) in dglyphs {
        lookup.visit_glyph(*v, |vg| {
            reverse_gmap.insert((*c, Variant::Ssty(0)), vg);
        });
    }
}

pub(crate) fn load_math_variants(
    reverse_gmap: &mut ReverseGlyphMap,
    variant: &ttf_parser::math::Variants<'_>,
    dglyphs: &[(char, GlyphId)],
) {
    for (c, dglyph) in dglyphs {
        variant.visit_glyph(*dglyph, |vg| {
            reverse_gmap.insert((*c, Variant::Math), vg);
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Variant {
    Id,
    Ssty(u16),
    Cv(u16),
    Ss(u16),
    Math,
}

/// A collection for obtaining the usv's from glyphs
#[derive(Default, Debug)]
pub struct ReverseGlyphMap {
    inner: AHashMap<GlyphId, (char, Variant)>,
}

impl ReverseGlyphMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query_usv(&self, glyph: GlyphId) -> Option<(char, Variant)> {
        self.inner.get(&glyph).copied()
    }

    pub fn insert(&mut self, usv: (char, Variant), glyph: GlyphId) -> Option<(char, Variant)> {
        self.inner.insert(glyph, usv)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

pub(super) fn is_supported_tags(tag: Tag) -> bool {
    if tag == Tag::from_bytes(b"ssty") {
        return true;
    }

    let tag = tag.to_string();
    let (a, n) = tag.split_at(2);

    if a.starts_with("cv") {
        match n.parse::<u32>().map(|ref n| (1..=99).contains(n)) {
            Ok(true) => true,
            _ => false,
        }
    } else if a.starts_with("ss") {
        match n.parse::<u32>().map(|ref n| (1..=20).contains(n)) {
            Ok(true) => true,
            _ => false,
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_tags() {
        // Test for supported tags
        assert!(is_supported_tags(Tag::from_bytes(b"ssty")));
        assert!(is_supported_tags(Tag::from_bytes(b"cv01")));
        assert!(is_supported_tags(Tag::from_bytes(b"cv99")));
        assert!(is_supported_tags(Tag::from_bytes(b"ss01")));
        assert!(is_supported_tags(Tag::from_bytes(b"ss20")));

        // Test for unsupported tags
        assert!(!is_supported_tags(Tag::from_bytes(b"abcd")));
        assert!(!is_supported_tags(Tag::from_bytes(b"cv00")));
        assert!(!is_supported_tags(Tag::from_bytes(b"cvv0")));
        assert!(!is_supported_tags(Tag::from_bytes(b"ss00")));
        assert!(!is_supported_tags(Tag::from_bytes(b"ss21")));
    }
}

mod css;
