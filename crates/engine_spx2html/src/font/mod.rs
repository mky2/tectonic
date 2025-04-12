use ahash::AHashMap;

use ttf_parser::{
    gsub::{AlternateSubstitution, SingleSubstitution, SubstitutionSubtable},
    opentype_layout::{LayoutTable, Lookup},
    GlyphId, Tag,
};

trait ApplyVariant {
    fn apply(&mut self, variant: GlyphId);
}

impl<T: FnMut(GlyphId)> ApplyVariant for T {
    fn apply(&mut self, variant: GlyphId) {
        self(variant)
    }
}

trait VariantGlyphVisitor {
    /// Visit a variant glyph.
    fn visit_glyph(&self, dglyph: GlyphId, f: &mut dyn ApplyVariant) -> Option<()>;
}

impl VariantGlyphVisitor for AlternateSubstitution<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, f: &mut dyn ApplyVariant) -> Option<()> {
        self.coverage
            .get(dglyph)
            .and_then(|index| self.alternate_sets.get(index))
            .map(|ref a| a.alternates)
            .iter()
            .for_each(|set| {
                set.into_iter().for_each(|g| f.apply(g));
            });
        Some(())
    }
}

impl VariantGlyphVisitor for SingleSubstitution<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, f: &mut dyn ApplyVariant) -> Option<()> {
        match *self {
            SingleSubstitution::Format1 { coverage, delta } => {
                if let Some(_) = coverage.get(dglyph) {
                    f.apply(GlyphId((i32::from(dglyph.0) + i32::from(delta)) as u16));
                }
            }
            SingleSubstitution::Format2 {
                coverage,
                substitutes,
            } => {
                if let Some(index) = coverage.get(dglyph) {
                    f.apply(substitutes.get(index)?);
                }
            }
        }
        Some(())
    }
}

impl VariantGlyphVisitor for ttf_parser::math::Variants<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, f: &mut dyn ApplyVariant) -> Option<()> {
        // Do we need to pass on whether a variant glyph is horizontal or vertical to upstream?

        if let Some(hvars) = self
            .horizontal_constructions
            .get(dglyph)
            .map(|ref gc| gc.variants)
        {
            for hvar in hvars {
                f.apply(hvar.variant_glyph);
            }
        }

        if let Some(vvars) = self
            .vertical_constructions
            .get(dglyph)
            .map(|ref gc| gc.variants)
        {
            for vvar in vvars {
                f.apply(vvar.variant_glyph);
            }
        }

        Some(())
    }
}

impl VariantGlyphVisitor for Lookup<'_> {
    fn visit_glyph(&self, dglyph: GlyphId, f: &mut dyn ApplyVariant) -> Option<()> {
        use SubstitutionSubtable::*;

        for subtable in self.subtables.into_iter::<SubstitutionSubtable>() {
            match subtable {
                Single(t) => {
                    t.visit_glyph(dglyph, f);
                }
                Alternate(t) => {
                    t.visit_glyph(dglyph, f);
                }
                _ => {}
            }
        }
        Some(())
    }
}

pub(crate) fn load_gsub(
    reverse_gmap: &mut ReverseGlyphMap,
    gsub: &LayoutTable<'_>,
    dglyphs: &[(char, GlyphId)],
) -> Option<()> {
    for (c, dglyph) in dglyphs {
        for feat in gsub.features {
            let tag_variant = match get_tag_variant(feat.tag) {
                Some(e) => e,
                None => continue,
            };

            for lookup_idx in feat.lookup_indices {
                if let Some(ref lookup) = gsub.lookups.get(lookup_idx) {
                    lookup.visit_glyph(*dglyph, &mut |vg| {
                        reverse_gmap.insert((*c, tag_variant), vg);
                    });
                }
            }
        }
    }
    Some(())
}

pub(crate) fn load_math_variants(
    reverse_gmap: &mut ReverseGlyphMap,
    variant: &ttf_parser::math::Variants<'_>,
    dglyphs: &[(char, GlyphId)],
) -> Option<()> {
    for (c, dglyph) in dglyphs {
        variant.visit_glyph(*dglyph, &mut |vg| {
            reverse_gmap.insert((*c, Variant::Math), vg);
        });
    }
    Some(())
}

#[derive(Debug, Clone, Copy)]
pub enum Variant {
    Direct,
    // https://learn.microsoft.com/en-us/typography/opentype/spec/features_pt#tag-ssty
    Ssty,
    // https://learn.microsoft.com/en-us/typography/opentype/spec/math
    Math,
    // https://learn.microsoft.com/en-us/typography/opentype/spec/features_ae#tag-cv01--cv99
    CharacterVariant(u16),
    // https://learn.microsoft.com/en-us/typography/opentype/spec/features_pt#tag-ss01---ss20
    StylisticSet(u16),
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

fn get_tag_variant(tag: Tag) -> Option<Variant> {
    match tag.to_string().as_str() {
        "ssty" => Some(Variant::Ssty),
        tag if tag.starts_with("cv") => tag[2..]
            .parse::<u16>()
            .ok()
            .filter(|&n| (1..=99).contains(&n))
            .map(Variant::CharacterVariant),
        tag if tag.starts_with("ss") => tag[2..]
            .parse::<u16>()
            .ok()
            .filter(|&n| (1..=20).contains(&n))
            .map(Variant::StylisticSet),
        _ => None,
    }
}

mod css;
