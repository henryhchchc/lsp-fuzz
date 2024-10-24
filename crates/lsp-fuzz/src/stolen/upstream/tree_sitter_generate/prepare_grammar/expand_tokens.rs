use std::{collections::HashMap, sync::LazyLock};

use anyhow::{anyhow, Context, Result};
use regex_syntax::ast::{
    parse, Ast, ClassPerlKind, ClassSet, ClassSetBinaryOpKind, ClassSetItem, ClassUnicodeKind,
    RepetitionKind, RepetitionRange,
};

use super::ExtractedLexicalGrammar;
use crate::stolen::upstream::tree_sitter_generate::{
    grammars::{LexicalGrammar, LexicalVariable},
    nfa::{CharacterSet, Nfa, NfaState},
    rules::{Precedence, Rule},
};

static UNICODE_CATEGORIES: LazyLock<HashMap<&'static str, Vec<u32>>> =
    LazyLock::new(|| serde_json::from_str(UNICODE_CATEGORIES_JSON).unwrap());
static UNICODE_PROPERTIES: LazyLock<HashMap<&'static str, Vec<u32>>> =
    LazyLock::new(|| serde_json::from_str(UNICODE_PROPERTIES_JSON).unwrap());
static UNICODE_CATEGORY_ALIASES: LazyLock<HashMap<&'static str, String>> =
    LazyLock::new(|| serde_json::from_str(UNICODE_CATEGORY_ALIASES_JSON).unwrap());
static UNICODE_PROPERTY_ALIASES: LazyLock<HashMap<&'static str, String>> =
    LazyLock::new(|| serde_json::from_str(UNICODE_PROPERTY_ALIASES_JSON).unwrap());

const UNICODE_CATEGORIES_JSON: &str = include_str!("./unicode-categories.json");
const UNICODE_PROPERTIES_JSON: &str = include_str!("./unicode-properties.json");
const UNICODE_CATEGORY_ALIASES_JSON: &str = include_str!("./unicode-category-aliases.json");
const UNICODE_PROPERTY_ALIASES_JSON: &str = include_str!("./unicode-property-aliases.json");

struct NfaBuilder {
    nfa: Nfa,
    is_sep: bool,
    precedence_stack: Vec<i32>,
}

fn get_implicit_precedence(rule: &Rule) -> i32 {
    match rule {
        Rule::String(_) => 2,
        Rule::Metadata { rule, params } => {
            if params.is_main_token {
                get_implicit_precedence(rule) + 1
            } else {
                get_implicit_precedence(rule)
            }
        }
        _ => 0,
    }
}

const fn get_completion_precedence(rule: &Rule) -> i32 {
    if let Rule::Metadata { params, .. } = rule {
        if let Precedence::Integer(p) = params.precedence {
            return p;
        }
    }
    0
}

pub fn expand_tokens(mut grammar: ExtractedLexicalGrammar) -> Result<LexicalGrammar> {
    let mut builder = NfaBuilder {
        nfa: Nfa::new(),
        is_sep: true,
        precedence_stack: vec![0],
    };

    let separator_rule = if grammar.separators.is_empty() {
        Rule::Blank
    } else {
        grammar.separators.push(Rule::Blank);
        Rule::repeat(Rule::choice(grammar.separators))
    };

    let mut variables = Vec::new();
    for (i, variable) in grammar.variables.into_iter().enumerate() {
        let is_immediate_token = match &variable.rule {
            Rule::Metadata { params, .. } => params.is_main_token,
            _ => false,
        };

        builder.is_sep = false;
        builder.nfa.states.push(NfaState::Accept {
            variable_index: i,
            precedence: get_completion_precedence(&variable.rule),
        });
        let last_state_id = builder.nfa.last_state_id();
        builder
            .expand_rule(&variable.rule, last_state_id)
            .with_context(|| format!("Error processing rule {}", variable.name))?;

        if !is_immediate_token {
            builder.is_sep = true;
            let last_state_id = builder.nfa.last_state_id();
            builder.expand_rule(&separator_rule, last_state_id)?;
        }

        variables.push(LexicalVariable {
            name: variable.name,
            kind: variable.kind,
            implicit_precedence: get_implicit_precedence(&variable.rule),
            start_state: builder.nfa.last_state_id(),
        });
    }

    Ok(LexicalGrammar {
        nfa: builder.nfa,
        variables,
    })
}

impl NfaBuilder {
    fn expand_rule(&mut self, rule: &Rule, mut next_state_id: u32) -> Result<bool> {
        match rule {
            Rule::Pattern(s, f) => {
                let ast = parse::Parser::new().parse(s)?;
                self.expand_regex(&ast, next_state_id, f.contains('i'))
            }
            Rule::String(s) => {
                for c in s.chars().rev() {
                    self.push_advance(CharacterSet::empty().add_char(c), next_state_id);
                    next_state_id = self.nfa.last_state_id();
                }
                Ok(!s.is_empty())
            }
            Rule::Choice(elements) => {
                let mut alternative_state_ids = Vec::new();
                for element in elements {
                    if self.expand_rule(element, next_state_id)? {
                        alternative_state_ids.push(self.nfa.last_state_id());
                    } else {
                        alternative_state_ids.push(next_state_id);
                    }
                }
                alternative_state_ids.sort_unstable();
                alternative_state_ids.dedup();
                alternative_state_ids.retain(|i| *i != self.nfa.last_state_id());
                for alternative_state_id in alternative_state_ids {
                    self.push_split(alternative_state_id);
                }
                Ok(true)
            }
            Rule::Seq(elements) => {
                let mut result = false;
                for element in elements.iter().rev() {
                    if self.expand_rule(element, next_state_id)? {
                        result = true;
                    }
                    next_state_id = self.nfa.last_state_id();
                }
                Ok(result)
            }
            Rule::Repeat(rule) => {
                self.nfa.states.push(NfaState::Accept {
                    variable_index: 0,
                    precedence: 0,
                }); // Placeholder for split
                let split_state_id = self.nfa.last_state_id();
                if self.expand_rule(rule, split_state_id)? {
                    self.nfa.states[split_state_id as usize] =
                        NfaState::Split(self.nfa.last_state_id(), next_state_id);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Rule::Metadata { rule, params } => {
                let has_precedence = if let Precedence::Integer(precedence) = &params.precedence {
                    self.precedence_stack.push(*precedence);
                    true
                } else {
                    false
                };
                let result = self.expand_rule(rule, next_state_id);
                if has_precedence {
                    self.precedence_stack.pop();
                }
                result
            }
            Rule::Blank => Ok(false),
            _ => Err(anyhow!("Grammar error: Unexpected rule {rule:?}")),
        }
    }

    fn expand_regex(
        &mut self,
        ast: &Ast,
        mut next_state_id: u32,
        case_insensitive: bool,
    ) -> Result<bool> {
        const fn inverse_char(c: char) -> char {
            match c {
                'a'..='z' => (c as u8 - b'a' + b'A') as char,
                'A'..='Z' => (c as u8 - b'A' + b'a') as char,
                c => c,
            }
        }

        fn with_inverse_char(mut chars: CharacterSet) -> CharacterSet {
            for char in chars.clone().chars() {
                let inverted = inverse_char(char);
                if char != inverted {
                    chars = chars.add_char(inverted);
                }
            }
            chars
        }

        match ast {
            Ast::Empty(_) => Ok(false),
            Ast::Flags(_) => Err(anyhow!("Regex error: Flags are not supported")),
            Ast::Literal(literal) => {
                let mut char_set = CharacterSet::from_char(literal.c);
                if case_insensitive {
                    let inverted = inverse_char(literal.c);
                    if literal.c != inverted {
                        char_set = char_set.add_char(inverted);
                    }
                }
                self.push_advance(char_set, next_state_id);
                Ok(true)
            }
            Ast::Dot(_) => {
                self.push_advance(CharacterSet::from_char('\n').negate(), next_state_id);
                Ok(true)
            }
            Ast::Assertion(_) => Err(anyhow!("Regex error: Assertions are not supported")),
            Ast::ClassUnicode(class) => {
                let mut chars = self.expand_unicode_character_class(&class.kind)?;
                if class.negated {
                    chars = chars.negate();
                }
                if case_insensitive {
                    chars = with_inverse_char(chars);
                }
                self.push_advance(chars, next_state_id);
                Ok(true)
            }
            Ast::ClassPerl(class) => {
                let mut chars = self.expand_perl_character_class(&class.kind);
                if class.negated {
                    chars = chars.negate();
                }
                if case_insensitive {
                    chars = with_inverse_char(chars);
                }
                self.push_advance(chars, next_state_id);
                Ok(true)
            }
            Ast::ClassBracketed(class) => {
                let mut chars = self.translate_class_set(&class.kind)?;
                if class.negated {
                    chars = chars.negate();
                }
                if case_insensitive {
                    chars = with_inverse_char(chars);
                }
                self.push_advance(chars, next_state_id);
                Ok(true)
            }
            Ast::Repetition(repetition) => match repetition.op.kind {
                RepetitionKind::ZeroOrOne => {
                    self.expand_zero_or_one(&repetition.ast, next_state_id, case_insensitive)
                }
                RepetitionKind::OneOrMore => {
                    self.expand_one_or_more(&repetition.ast, next_state_id, case_insensitive)
                }
                RepetitionKind::ZeroOrMore => {
                    self.expand_zero_or_more(&repetition.ast, next_state_id, case_insensitive)
                }
                RepetitionKind::Range(RepetitionRange::Exactly(count)) => {
                    self.expand_count(&repetition.ast, count, next_state_id, case_insensitive)
                }
                RepetitionKind::Range(RepetitionRange::AtLeast(min)) => {
                    if self.expand_zero_or_more(&repetition.ast, next_state_id, case_insensitive)? {
                        self.expand_count(&repetition.ast, min, next_state_id, case_insensitive)
                    } else {
                        Ok(false)
                    }
                }
                RepetitionKind::Range(RepetitionRange::Bounded(min, max)) => {
                    let mut result =
                        self.expand_count(&repetition.ast, min, next_state_id, case_insensitive)?;
                    for _ in min..max {
                        if result {
                            next_state_id = self.nfa.last_state_id();
                        }
                        if self.expand_zero_or_one(
                            &repetition.ast,
                            next_state_id,
                            case_insensitive,
                        )? {
                            result = true;
                        }
                    }
                    Ok(result)
                }
            },
            Ast::Group(group) => self.expand_regex(&group.ast, next_state_id, case_insensitive),
            Ast::Alternation(alternation) => {
                let mut alternative_state_ids = Vec::new();
                for ast in &alternation.asts {
                    if self.expand_regex(ast, next_state_id, case_insensitive)? {
                        alternative_state_ids.push(self.nfa.last_state_id());
                    } else {
                        alternative_state_ids.push(next_state_id);
                    }
                }
                alternative_state_ids.sort_unstable();
                alternative_state_ids.dedup();
                alternative_state_ids.retain(|i| *i != self.nfa.last_state_id());

                for alternative_state_id in alternative_state_ids {
                    self.push_split(alternative_state_id);
                }
                Ok(true)
            }
            Ast::Concat(concat) => {
                let mut result = false;
                for ast in concat.asts.iter().rev() {
                    if self.expand_regex(ast, next_state_id, case_insensitive)? {
                        result = true;
                        next_state_id = self.nfa.last_state_id();
                    }
                }
                Ok(result)
            }
        }
    }

    fn translate_class_set(&self, class_set: &ClassSet) -> Result<CharacterSet> {
        match &class_set {
            ClassSet::Item(item) => self.expand_character_class(item),
            ClassSet::BinaryOp(binary_op) => {
                let mut lhs_char_class = self.translate_class_set(&binary_op.lhs)?;
                let mut rhs_char_class = self.translate_class_set(&binary_op.rhs)?;
                match binary_op.kind {
                    ClassSetBinaryOpKind::Intersection => {
                        Ok(lhs_char_class.remove_intersection(&mut rhs_char_class))
                    }
                    ClassSetBinaryOpKind::Difference => {
                        Ok(lhs_char_class.difference(rhs_char_class))
                    }
                    ClassSetBinaryOpKind::SymmetricDifference => {
                        Ok(lhs_char_class.symmetric_difference(rhs_char_class))
                    }
                }
            }
        }
    }

    fn expand_one_or_more(
        &mut self,
        ast: &Ast,
        next_state_id: u32,
        case_insensitive: bool,
    ) -> Result<bool> {
        self.nfa.states.push(NfaState::Accept {
            variable_index: 0,
            precedence: 0,
        }); // Placeholder for split
        let split_state_id = self.nfa.last_state_id();
        if self.expand_regex(ast, split_state_id, case_insensitive)? {
            self.nfa.states[split_state_id as usize] =
                NfaState::Split(self.nfa.last_state_id(), next_state_id);
            Ok(true)
        } else {
            self.nfa.states.pop();
            Ok(false)
        }
    }

    fn expand_zero_or_one(
        &mut self,
        ast: &Ast,
        next_state_id: u32,
        case_insensitive: bool,
    ) -> Result<bool> {
        if self.expand_regex(ast, next_state_id, case_insensitive)? {
            self.push_split(next_state_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expand_zero_or_more(
        &mut self,
        ast: &Ast,
        next_state_id: u32,
        case_insensitive: bool,
    ) -> Result<bool> {
        if self.expand_one_or_more(ast, next_state_id, case_insensitive)? {
            self.push_split(next_state_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expand_count(
        &mut self,
        ast: &Ast,
        count: u32,
        mut next_state_id: u32,
        case_insensitive: bool,
    ) -> Result<bool> {
        let mut result = false;
        for _ in 0..count {
            if self.expand_regex(ast, next_state_id, case_insensitive)? {
                result = true;
                next_state_id = self.nfa.last_state_id();
            }
        }
        Ok(result)
    }

    fn expand_character_class(&self, item: &ClassSetItem) -> Result<CharacterSet> {
        match item {
            ClassSetItem::Empty(_) => Ok(CharacterSet::empty()),
            ClassSetItem::Literal(literal) => Ok(CharacterSet::from_char(literal.c)),
            ClassSetItem::Range(range) => Ok(CharacterSet::from_range(range.start.c, range.end.c)),
            ClassSetItem::Union(union) => {
                let mut result = CharacterSet::empty();
                for item in &union.items {
                    result = result.add(&self.expand_character_class(item)?);
                }
                Ok(result)
            }
            ClassSetItem::Perl(class) => Ok(self.expand_perl_character_class(&class.kind)),
            ClassSetItem::Unicode(class) => {
                let mut set = self.expand_unicode_character_class(&class.kind)?;
                if class.negated {
                    set = set.negate();
                }
                Ok(set)
            }
            ClassSetItem::Bracketed(class) => {
                let mut set = self.translate_class_set(&class.kind)?;
                if class.negated {
                    set = set.negate();
                }
                Ok(set)
            }
            ClassSetItem::Ascii(_) => Err(anyhow!(
                "Regex error: Unsupported character class syntax {item:?}",
            )),
        }
    }

    fn expand_unicode_character_class(&self, class: &ClassUnicodeKind) -> Result<CharacterSet> {
        let mut chars = CharacterSet::empty();

        let category_letter;
        match class {
            ClassUnicodeKind::OneLetter(le) => {
                category_letter = le.to_string();
            }
            ClassUnicodeKind::Named(class_name) => {
                let actual_class_name = UNICODE_CATEGORY_ALIASES
                    .get(class_name.as_str())
                    .or_else(|| UNICODE_PROPERTY_ALIASES.get(class_name.as_str()))
                    .unwrap_or(class_name);
                if actual_class_name.len() == 1 {
                    category_letter = actual_class_name.clone();
                } else {
                    let code_points =
                        UNICODE_CATEGORIES
                            .get(actual_class_name.as_str())
                            .or_else(|| UNICODE_PROPERTIES.get(actual_class_name.as_str()))
                            .ok_or_else(|| {
                                anyhow!(
                                    "Regex error: Unsupported unicode character class {class_name}",
                                )
                            })?;
                    for c in code_points {
                        if let Some(c) = char::from_u32(*c) {
                            chars = chars.add_char(c);
                        }
                    }

                    return Ok(chars);
                }
            }
            ClassUnicodeKind::NamedValue { .. } => {
                return Err(anyhow!(
                    "Regex error: Key-value unicode properties are not supported"
                ))
            }
        }

        for (category, code_points) in UNICODE_CATEGORIES.iter() {
            if category.starts_with(&category_letter) {
                for c in code_points {
                    if let Some(c) = char::from_u32(*c) {
                        chars = chars.add_char(c);
                    }
                }
            }
        }

        Ok(chars)
    }

    fn expand_perl_character_class(&self, item: &ClassPerlKind) -> CharacterSet {
        match item {
            ClassPerlKind::Digit => CharacterSet::from_range('0', '9'),
            ClassPerlKind::Space => CharacterSet::empty()
                .add_char(' ')
                .add_char('\t')
                .add_char('\r')
                .add_char('\n')
                .add_char('\x0B')
                .add_char('\x0C'),
            ClassPerlKind::Word => CharacterSet::empty()
                .add_char('_')
                .add_range('A', 'Z')
                .add_range('a', 'z')
                .add_range('0', '9'),
        }
    }

    fn push_advance(&mut self, chars: CharacterSet, state_id: u32) {
        let precedence = *self.precedence_stack.last().unwrap();
        self.nfa.states.push(NfaState::Advance {
            chars,
            state_id,
            precedence,
            is_sep: self.is_sep,
        });
    }

    fn push_split(&mut self, state_id: u32) {
        let last_state_id = self.nfa.last_state_id();
        self.nfa
            .states
            .push(NfaState::Split(state_id, last_state_id));
    }
}
