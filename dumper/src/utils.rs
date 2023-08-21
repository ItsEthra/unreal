use crate::{
    engine::{BoolVars, FBoolProperty, FPropertyPtr},
    sdk::PropertyKind,
};
use anyhow::Result;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display},
    mem::take,
};

#[macro_export]
macro_rules! fqn {
    ($ident:expr) => {
        $crate::utils::Fqn::new($ident)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    pub size: usize,
    pub align: usize,
}

impl Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Size = {:#X}, Align = {:#X}", self.size, self.align)
    }
}

impl Layout {
    pub fn same(value: usize) -> Layout {
        assert!(value != 0, "Layout must not be 0");
        Layout {
            size: value,
            align: value,
        }
    }

    pub fn get_aligned_size(&self) -> usize {
        let mut aligned = self.size;
        if aligned % self.align.max(1) != 0 {
            aligned += self.align - aligned % self.align;
        }

        aligned
    }
}

#[test]
fn test_align_layout() {
    let layout = Layout { size: 12, align: 8 };
    assert_eq!(layout.get_aligned_size(), 16);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fqn {
    package: &'static str,
    name: &'static str,
}

impl Fqn {
    pub fn new(ident: &'static str) -> Self {
        let (package, name) = ident.split_once('.').expect("Invalid FQN");
        Self { package, name }
    }

    pub fn eq_str(&self, s: &str) -> bool {
        s.strip_prefix(self.package)
            .and_then(|s| s.strip_suffix(self.name))
            .map(|s| s == ".")
            .unwrap_or(false)
    }

    pub const fn from_package_name(package: &'static str, name: &'static str) -> Self {
        Self { package, name }
    }

    pub const fn package(&self) -> &'static str {
        self.package
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.package, self.name)
    }
}

impl Debug for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.package, self.name)
    }
}

#[derive(Debug)]
pub struct Bitfield {
    pub name: String,
    // Bit offset
    pub offset: u32,
    // Bit length
    pub len: u32,
}

// Although groups might be bigger than 1 byte, I've never seen such a thing.
#[derive(Debug)]
pub struct BitfieldGroup {
    pub offset: usize,
    pub items: Vec<Bitfield>,
}

#[derive(Default)]
pub struct BitfieldAccumulator {
    current: Option<BitfieldGroup>,
    groups: Vec<BitfieldGroup>,
}

impl BitfieldAccumulator {
    pub fn accumulate(
        &mut self,
        name: &str,
        fproperty: FPropertyPtr,
        kind: &PropertyKind,
        offset: usize,
    ) -> Result<AccumulatorResult> {
        let result = if matches!(kind, PropertyKind::Bool) {
            let BoolVars {
                byte_mask,
                field_mask,
                ..
            } = fproperty.cast::<FBoolProperty>().vars()?;

            let make_field = || Bitfield {
                len: (byte_mask >> byte_mask.trailing_zeros()).trailing_ones(),
                offset: byte_mask.trailing_zeros(),
                name: name.to_owned(),
            };

            if field_mask == 255 {
                // Field is a normal bool, just yield currently accumulated groups.
                if let Some(current) = self.current.take() {
                    self.groups.push(current);
                }

                AccumulatorResult::Append(take(&mut self.groups))
            } else if let Some(ref mut current) = self.current {
                // Field is a bitfield which might not necesserily by in the current group.
                if current.offset == offset {
                    // Field should be in the current group
                    current.items.push(make_field());
                    AccumulatorResult::Skip
                } else {
                    // Field is a start of a new group.
                    self.groups.push(self.current.take().unwrap());
                    self.current = Some(BitfieldGroup {
                        offset,
                        items: vec![make_field()],
                    });
                    AccumulatorResult::Skip
                }
            } else {
                // Field is a bitfield, with no old group which means it's a start of a new group.
                self.current = Some(BitfieldGroup {
                    offset,
                    items: vec![make_field()],
                });

                AccumulatorResult::Skip
            }
        } else {
            // Field is not a bool, certainly not a bitfield, then we just yield currently accumulated groups.
            if let Some(current) = self.current.take() {
                self.groups.push(current);
            }

            AccumulatorResult::Append(take(&mut self.groups))
        };

        Ok(result)
    }
}

#[derive(Debug)]
pub enum AccumulatorResult {
    Append(Vec<BitfieldGroup>),
    Skip,
}

pub fn strip_package_name(pkg: &str) -> &str {
    pkg.rsplit_once('/').map(|p| p.1).unwrap_or(pkg)
}

pub fn sanitize_ident(ident: &str) -> Cow<str> {
    if ident == "Self" {
        return "This".into();
    }

    let okay = |c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_');

    if ident.chars().any(|c| !okay(c)) {
        ident
            .chars()
            .map(|c| if okay(c) { c } else { '_' })
            .collect::<String>()
            .into()
    } else {
        Cow::Borrowed(ident)
    }
}
