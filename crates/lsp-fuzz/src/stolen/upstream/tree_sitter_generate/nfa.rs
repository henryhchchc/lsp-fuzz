use std::{
    cmp::Ordering,
    fmt::{self, Formatter},
    iter::ExactSizeIterator,
    mem::swap,
    ops::{Range, RangeInclusive},
};

/// A set of characters represented as a vector of ranges.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct CharacterSet {
    ranges: Vec<Range<u32>>,
}

/// A state in an NFA representing a regular grammar.
#[derive(Debug, PartialEq, Eq)]
pub enum NfaState {
    Advance {
        chars: CharacterSet,
        state_id: u32,
        is_sep: bool,
        precedence: i32,
    },
    Split(u32, u32),
    Accept {
        variable_index: usize,
        precedence: i32,
    },
}

#[derive(PartialEq, Eq, Default)]
pub struct Nfa {
    pub states: Vec<NfaState>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NfaTransition {
    pub characters: CharacterSet,
    pub is_separator: bool,
    pub precedence: i32,
    pub states: Vec<u32>,
}

const END: u32 = char::MAX as u32 + 1;

impl CharacterSet {
    /// Create a character set with a single character.
    pub const fn empty() -> Self {
        Self { ranges: Vec::new() }
    }

    /// Create a character set with a given *inclusive* range of characters.
    #[allow(clippy::single_range_in_vec_init)]
    pub fn from_range(mut first: char, mut last: char) -> Self {
        if first > last {
            swap(&mut first, &mut last);
        }
        Self {
            ranges: vec![(first as u32)..(last as u32 + 1)],
        }
    }

    /// Create a character set with a single character.
    #[allow(clippy::single_range_in_vec_init)]
    pub fn from_char(c: char) -> Self {
        Self {
            ranges: vec![(c as u32)..(c as u32 + 1)],
        }
    }

    /// Create a character set containing all characters *not* present
    /// in this character set.
    pub fn negate(mut self) -> Self {
        let mut i = 0;
        let mut previous_end = 0;
        while i < self.ranges.len() {
            let range = &mut self.ranges[i];
            let start = previous_end;
            previous_end = range.end;
            if start < range.start {
                self.ranges[i] = start..range.start;
                i += 1;
            } else {
                self.ranges.remove(i);
            }
        }
        if previous_end < END {
            self.ranges.push(previous_end..END);
        }
        self
    }

    pub fn add_char(mut self, c: char) -> Self {
        self.add_int_range(0, c as u32, c as u32 + 1);
        self
    }

    pub fn add_range(mut self, start: char, end: char) -> Self {
        self.add_int_range(0, start as u32, end as u32 + 1);
        self
    }

    pub fn add(mut self, other: &Self) -> Self {
        let mut index = 0;
        for range in &other.ranges {
            index = self.add_int_range(index, range.start, range.end);
        }
        self
    }

    fn add_int_range(&mut self, mut i: usize, start: u32, end: u32) -> usize {
        while i < self.ranges.len() {
            let range = &mut self.ranges[i];
            if range.start > end {
                self.ranges.insert(i, start..end);
                return i;
            }
            if range.end >= start {
                range.end = range.end.max(end);
                range.start = range.start.min(start);

                // Join this range with the next range if needed.
                while i + 1 < self.ranges.len() && self.ranges[i + 1].start <= self.ranges[i].end {
                    self.ranges[i].end = self.ranges[i].end.max(self.ranges[i + 1].end);
                    self.ranges.remove(i + 1);
                }

                return i;
            }
            i += 1;
        }
        self.ranges.push(start..end);
        i
    }

    /// Get the set of characters that are present in both this set
    /// and the other set. Remove those common characters from both
    /// of the operands.
    pub fn remove_intersection(&mut self, other: &mut Self) -> Self {
        let mut intersection = Vec::new();
        let mut left_i = 0;
        let mut right_i = 0;
        while left_i < self.ranges.len() && right_i < other.ranges.len() {
            let left = &mut self.ranges[left_i];
            let right = &mut other.ranges[right_i];

            match left.start.cmp(&right.start) {
                Ordering::Less => {
                    // [ L ]
                    //     [ R ]
                    if left.end <= right.start {
                        left_i += 1;
                        continue;
                    }

                    match left.end.cmp(&right.end) {
                        // [ L ]
                        //   [ R ]
                        Ordering::Less => {
                            intersection.push(right.start..left.end);
                            swap(&mut left.end, &mut right.start);
                            left_i += 1;
                        }

                        // [  L  ]
                        //   [ R ]
                        Ordering::Equal => {
                            intersection.push(right.clone());
                            left.end = right.start;
                            other.ranges.remove(right_i);
                        }

                        // [   L   ]
                        //   [ R ]
                        Ordering::Greater => {
                            intersection.push(right.clone());
                            let new_range = left.start..right.start;
                            left.start = right.end;
                            self.ranges.insert(left_i, new_range);
                            other.ranges.remove(right_i);
                            left_i += 1;
                        }
                    }
                }
                // [ L ]
                // [  R  ]
                Ordering::Equal if left.end < right.end => {
                    intersection.push(left.start..left.end);
                    right.start = left.end;
                    self.ranges.remove(left_i);
                }
                // [ L ]
                // [ R ]
                Ordering::Equal if left.end == right.end => {
                    intersection.push(left.clone());
                    self.ranges.remove(left_i);
                    other.ranges.remove(right_i);
                }
                // [  L  ]
                // [ R ]
                Ordering::Equal if left.end > right.end => {
                    intersection.push(right.clone());
                    left.start = right.end;
                    other.ranges.remove(right_i);
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    //     [ L ]
                    // [ R ]
                    if left.start >= right.end {
                        right_i += 1;
                        continue;
                    }

                    match left.end.cmp(&right.end) {
                        //   [ L ]
                        // [   R   ]
                        Ordering::Less => {
                            intersection.push(left.clone());
                            let new_range = right.start..left.start;
                            right.start = left.end;
                            other.ranges.insert(right_i, new_range);
                            self.ranges.remove(left_i);
                            right_i += 1;
                        }

                        //   [ L ]
                        // [  R  ]
                        Ordering::Equal => {
                            intersection.push(left.clone());
                            right.end = left.start;
                            self.ranges.remove(left_i);
                        }

                        //   [   L   ]
                        // [   R   ]
                        Ordering::Greater => {
                            intersection.push(left.start..right.end);
                            swap(&mut left.start, &mut right.end);
                            right_i += 1;
                        }
                    }
                }
            }
        }
        Self {
            ranges: intersection,
        }
    }

    /// Produces a `CharacterSet` containing every character in `self` that is not present in
    /// `other`.
    pub fn difference(mut self, mut other: Self) -> Self {
        self.remove_intersection(&mut other);
        self
    }

    /// Produces a `CharacterSet` containing every character that is in _exactly one_ of `self` or
    /// `other`, but is not present in both sets.
    pub fn symmetric_difference(mut self, mut other: Self) -> Self {
        self.remove_intersection(&mut other);
        self.add(&other)
    }

    pub fn char_codes(&self) -> impl Iterator<Item = u32> + '_ {
        self.ranges.iter().flat_map(Clone::clone)
    }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.char_codes().filter_map(char::from_u32)
    }

    pub fn range_count(&self) -> usize {
        self.ranges.len()
    }

    pub fn ranges(&self) -> impl Iterator<Item = RangeInclusive<char>> + '_ {
        self.ranges.iter().filter_map(|range| {
            let start = range.clone().find_map(char::from_u32)?;
            let end = (range.start..range.end).rev().find_map(char::from_u32)?;
            Some(start..=end)
        })
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Get a reduced list of character ranges, assuming that a given
    /// set of characters can be safely ignored.
    pub fn simplify_ignoring(&self, ruled_out_characters: &Self) -> Self {
        let mut prev_range: Option<Range<u32>> = None;
        Self {
            ranges: self
                .ranges
                .iter()
                .map(|range| Some(range.clone()))
                .chain([None])
                .filter_map(move |range| {
                    if let Some(range) = &range {
                        if ruled_out_characters.contains_codepoint_range(range.clone()) {
                            return None;
                        }

                        if let Some(prev_range) = &mut prev_range {
                            if ruled_out_characters
                                .contains_codepoint_range(prev_range.end..range.start)
                            {
                                prev_range.end = range.end;
                                return None;
                            }
                        }
                    }

                    let result = prev_range.clone();
                    prev_range = range;
                    result
                })
                .collect(),
        }
    }

    pub fn contains_codepoint_range(&self, seek_range: Range<u32>) -> bool {
        let ix = match self.ranges.binary_search_by(|probe| {
            if probe.end <= seek_range.start {
                Ordering::Less
            } else if probe.start > seek_range.start {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }) {
            Ok(ix) | Err(ix) => ix,
        };
        self.ranges.get(ix).map_or(false, |range| {
            range.start <= seek_range.start && range.end >= seek_range.end
        })
    }

    pub fn contains(&self, c: char) -> bool {
        self.contains_codepoint_range(c as u32..c as u32 + 1)
    }
}

impl Ord for CharacterSet {
    fn cmp(&self, other: &Self) -> Ordering {
        let count_cmp = self
            .ranges
            .iter()
            .map(ExactSizeIterator::len)
            .sum::<usize>()
            .cmp(&other.ranges.iter().map(ExactSizeIterator::len).sum());
        if count_cmp != Ordering::Equal {
            return count_cmp;
        }

        for (left_range, right_range) in self.ranges.iter().zip(other.ranges.iter()) {
            let cmp = left_range.len().cmp(&right_range.len());
            if cmp != Ordering::Equal {
                return cmp;
            }

            for (left, right) in left_range.clone().zip(right_range.clone()) {
                let cmp = left.cmp(&right);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for CharacterSet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for CharacterSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "CharacterSet [")?;
        let mut set = self.clone();
        if self.contains(char::MAX) {
            write!(f, "^ ")?;
            set = set.negate();
        }
        for (i, range) in set.ranges().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{range:?}")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Nfa {
    #[must_use]
    pub const fn new() -> Self {
        Self { states: Vec::new() }
    }

    pub fn last_state_id(&self) -> u32 {
        self.states.len() as u32 - 1
    }
}

impl fmt::Debug for Nfa {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Nfa {{ states: {{")?;
        for (i, state) in self.states.iter().enumerate() {
            writeln!(f, "  {i}: {state:?},")?;
        }
        write!(f, "}} }}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adding_ranges() {
        let mut set = CharacterSet::empty()
            .add_range('c', 'm')
            .add_range('q', 's');

        // within existing range
        set = set.add_char('d');
        assert_eq!(
            set,
            CharacterSet::empty()
                .add_range('c', 'm')
                .add_range('q', 's')
        );

        // at end of existing range
        set = set.add_char('m');
        assert_eq!(
            set,
            CharacterSet::empty()
                .add_range('c', 'm')
                .add_range('q', 's')
        );

        // adjacent to end of existing range
        set = set.add_char('n');
        assert_eq!(
            set,
            CharacterSet::empty()
                .add_range('c', 'n')
                .add_range('q', 's')
        );

        // filling gap between existing ranges
        set = set.add_range('o', 'p');
        assert_eq!(set, CharacterSet::empty().add_range('c', 's'));

        set = CharacterSet::empty()
            .add_range('c', 'f')
            .add_range('i', 'l')
            .add_range('n', 'r');
        set = set.add_range('d', 'o');
        assert_eq!(set, CharacterSet::empty().add_range('c', 'r'));
    }

    #[test]
    fn test_adding_sets() {
        let set1 = CharacterSet::empty()
            .add_range('c', 'f')
            .add_range('i', 'l');
        let set2 = CharacterSet::empty().add_range('b', 'g').add_char('h');
        assert_eq!(
            set1.add(&set2),
            CharacterSet::empty()
                .add_range('b', 'g')
                .add_range('h', 'l')
        );
    }

    #[test]
    fn test_character_set_intersection_difference_ops() {
        struct Row {
            left: CharacterSet,
            right: CharacterSet,
            left_only: CharacterSet,
            right_only: CharacterSet,
            intersection: CharacterSet,
        }

        let rows = [
            // [ L ]
            //     [ R ]
            Row {
                left: CharacterSet::from_range('a', 'f'),
                right: CharacterSet::from_range('g', 'm'),
                left_only: CharacterSet::from_range('a', 'f'),
                right_only: CharacterSet::from_range('g', 'm'),
                intersection: CharacterSet::empty(),
            },
            // [ L ]
            //   [ R ]
            Row {
                left: CharacterSet::from_range('a', 'f'),
                right: CharacterSet::from_range('c', 'i'),
                left_only: CharacterSet::from_range('a', 'b'),
                right_only: CharacterSet::from_range('g', 'i'),
                intersection: CharacterSet::from_range('c', 'f'),
            },
            // [  L  ]
            //   [ R ]
            Row {
                left: CharacterSet::from_range('a', 'f'),
                right: CharacterSet::from_range('d', 'f'),
                left_only: CharacterSet::from_range('a', 'c'),
                right_only: CharacterSet::empty(),
                intersection: CharacterSet::from_range('d', 'f'),
            },
            // [   L   ]
            //   [ R ]
            Row {
                left: CharacterSet::from_range('a', 'm'),
                right: CharacterSet::from_range('d', 'f'),
                left_only: CharacterSet::empty()
                    .add_range('a', 'c')
                    .add_range('g', 'm'),
                right_only: CharacterSet::empty(),
                intersection: CharacterSet::from_range('d', 'f'),
            },
            // [    L    ]
            //         [R]
            Row {
                left: CharacterSet::from_range(',', '/'),
                right: CharacterSet::from_char('/'),
                left_only: CharacterSet::from_range(',', '.'),
                right_only: CharacterSet::empty(),
                intersection: CharacterSet::from_char('/'),
            },
            // [    L    ]
            //         [R]
            Row {
                left: CharacterSet::from_range(',', '/'),
                right: CharacterSet::from_char('/'),
                left_only: CharacterSet::from_range(',', '.'),
                right_only: CharacterSet::empty(),
                intersection: CharacterSet::from_char('/'),
            },
            // [ L1 ] [ L2 ]
            //    [  R  ]
            Row {
                left: CharacterSet::empty()
                    .add_range('a', 'e')
                    .add_range('h', 'l'),
                right: CharacterSet::from_range('c', 'i'),
                left_only: CharacterSet::empty()
                    .add_range('a', 'b')
                    .add_range('j', 'l'),
                right_only: CharacterSet::from_range('f', 'g'),
                intersection: CharacterSet::empty()
                    .add_range('c', 'e')
                    .add_range('h', 'i'),
            },
        ];

        for (i, row) in rows.iter().enumerate() {
            let mut left = row.left.clone();
            let mut right = row.right.clone();
            assert_eq!(
                left.remove_intersection(&mut right),
                row.intersection,
                "row {}a: {:?} && {:?}",
                i,
                row.left,
                row.right
            );
            assert_eq!(
                left, row.left_only,
                "row {}a: {:?} - {:?}",
                i, row.left, row.right
            );
            assert_eq!(
                right, row.right_only,
                "row {}a: {:?} - {:?}",
                i, row.right, row.left
            );

            let mut left = row.left.clone();
            let mut right = row.right.clone();
            assert_eq!(
                right.remove_intersection(&mut left),
                row.intersection,
                "row {}b: {:?} && {:?}",
                i,
                row.left,
                row.right
            );
            assert_eq!(
                left, row.left_only,
                "row {}b: {:?} - {:?}",
                i, row.left, row.right
            );
            assert_eq!(
                right, row.right_only,
                "row {}b: {:?} - {:?}",
                i, row.right, row.left
            );

            assert_eq!(
                row.left.clone().difference(row.right.clone()),
                row.left_only,
                "row {}b: {:?} -- {:?}",
                i,
                row.left,
                row.right
            );

            let symm_difference = row.left_only.clone().add(&row.right_only);
            assert_eq!(
                row.left.clone().symmetric_difference(row.right.clone()),
                symm_difference,
                "row {i}b: {:?} ~~ {:?}",
                row.left,
                row.right
            );
        }
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn test_character_set_simplify_ignoring() {
        struct Row {
            chars: Vec<char>,
            ruled_out_chars: Vec<char>,
            expected_ranges: Vec<Range<char>>,
        }

        let table = [
            Row {
                chars: vec!['a'],
                ruled_out_chars: vec![],
                expected_ranges: vec!['a'..'a'],
            },
            Row {
                chars: vec!['a', 'b', 'c', 'e', 'z'],
                ruled_out_chars: vec![],
                expected_ranges: vec!['a'..'c', 'e'..'e', 'z'..'z'],
            },
            Row {
                chars: vec!['a', 'b', 'c', 'e', 'h', 'z'],
                ruled_out_chars: vec!['d', 'f', 'g'],
                expected_ranges: vec!['a'..'h', 'z'..'z'],
            },
            Row {
                chars: vec!['a', 'b', 'c', 'g', 'h', 'i'],
                ruled_out_chars: vec!['d', 'j'],
                expected_ranges: vec!['a'..'c', 'g'..'i'],
            },
            Row {
                chars: vec!['c', 'd', 'e', 'g', 'h'],
                ruled_out_chars: vec!['a', 'b', 'c', 'd', 'e', 'f'],
                expected_ranges: vec!['g'..'h'],
            },
            Row {
                chars: vec!['I', 'N'],
                ruled_out_chars: vec!['A', 'I', 'N', 'Z'],
                expected_ranges: vec![],
            },
        ];

        for Row {
            chars,
            ruled_out_chars,
            expected_ranges,
        } in &table
        {
            let ruled_out_chars = ruled_out_chars
                .iter()
                .fold(CharacterSet::empty(), |set, c| set.add_char(*c));
            let mut set = CharacterSet::empty();
            for c in chars {
                set = set.add_char(*c);
            }
            let actual = set.simplify_ignoring(&ruled_out_chars);
            let expected = expected_ranges
                .iter()
                .fold(CharacterSet::empty(), |set, range| {
                    set.add_range(range.start, range.end)
                });
            assert_eq!(
                actual, expected,
                "chars: {chars:?}, ruled out chars: {ruled_out_chars:?}"
            );
        }
    }
}
