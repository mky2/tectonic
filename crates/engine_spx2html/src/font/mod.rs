use std::collections::HashMap;

use ttf_parser::{
    gsub::{AlternateSubstitution, SubstitutionSubtable},
    opentype_layout::Lookup,
    GlyphId,
};

trait SubstituteGlyphVisitor<D> {
    fn visit_glyph<F: FnMut(GlyphId, D)>(&self, glyph: GlyphId, consumer: F);
}

impl SubstituteGlyphVisitor<Variant> for Lookup<'_> {
    fn visit_glyph<F: FnMut(GlyphId, Variant)>(&self, glyph: GlyphId, mut consumer: F) {
        use SubstitutionSubtable::*;

        for st in self.subtables.into_iter::<SubstitutionSubtable>() {
            match st {
                Alternate(t) => t.visit_glyph(glyph, &mut consumer),
                _ => {}
            }
        }
    }
}

impl SubstituteGlyphVisitor<Variant> for AlternateSubstitution<'_> {
    fn visit_glyph<F: FnMut(GlyphId, Variant)>(&self, glyph: GlyphId, mut consumer: F) {
        self.coverage
            .get(glyph)
            .map(|id| self.alternate_sets.get(id).unwrap().alternates)
            .iter()
            .for_each(|set| {
                set.into_iter()
                    .enumerate()
                    .for_each(|(i, g)| consumer(g, Variant::Ssty(i as u16)));
            });
    }
}

// impl SubstituteGlyphSupplier for SingleSubstitution<'_> {
//     fn substitute_glyph<F: Fn(GlyphId)>(&self, glyph: GlyphId, consumer: F) {
//         match *self {
//             SingleSubstitution::Format1 { coverage, delta } => {
//                 coverage.get(glyph).map(|id| {
//                     glyph.0 + delta
//                 });
//             },
//             SingleSubstitution::Format2 { coverage, substitutes } => {
//
//             },
//         }
//     }
// }

impl SubstituteGlyphVisitor<Variant> for ttf_parser::math::Variants<'_> {
    fn visit_glyph<F: FnMut(GlyphId, Variant)>(&self, glyph: GlyphId, mut consumer: F) {
        if let Some(hvars) = self
            .horizontal_constructions
            .get(glyph)
            .map(|ref gc| gc.variants)
        {
            for hvar in hvars {
                consumer(hvar.variant_glyph, Variant::Math);
            }
        }

        if let Some(vvars) = self
            .vertical_constructions
            .get(glyph)
            .map(|ref gc| gc.variants)
        {
            for vvar in vvars {
                consumer(vvar.variant_glyph, Variant::Math);
            }
        }
    }
}

pub(crate) fn load_lookup(
    reverse_gmap: &mut ReverseGlyphMap,
    lookup: &Lookup<'_>,
    dglyphs: &[(char, GlyphId)],
) {
    for (c, v) in dglyphs {
        lookup.visit_glyph(*v, |vg, v| {
            reverse_gmap.insert((*c, v), vg);
        });
    }
}

pub(crate) fn load_math_variants(
    reverse_gmap: &mut ReverseGlyphMap,
    variant: &ttf_parser::math::Variants<'_>,
    dglyphs: &[(char, GlyphId)],
) {
    dglyphs.iter().for_each(|(c, g)| {
        variant.visit_glyph(*g, |vg, v| {
            reverse_gmap.insert((*c, v), vg);
        });
    });
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
    inner: HashMap<GlyphId, (char, Variant)>,
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
