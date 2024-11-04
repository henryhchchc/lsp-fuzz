use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    io::Read,
    num::ParseIntError,
    ops::Range,
    str::FromStr,
    string::FromUtf8Error,
};

use dot_structures::{Attribute, Edge, EdgeTy, Graph, Id, Node, NodeId, Stmt, Vertex};
use itertools::{Either, Itertools};

use super::tree::NodeIter;

/// Extracts derivation fragments from the given source code using the provided parser.
///
/// This function parses the provided source code and traverses the resulting parse tree
/// to extract fragments of the code associated with each node type. The fragments are
/// grouped by their node types and returned as a `HashMap`.
///
/// # Arguments
///
/// * `code` - A byte slice representing the source code to be parsed.
/// * `parser` - A mutable reference to a `tree_sitter::Parser` used to parse the code.
///
/// # Returns
///
/// A `HashMap` where the keys are static string slices representing the node types,
/// and the values are vectors of byte slices representing the fragments of the code
/// associated with each node type.
pub fn extract_derivation_fragments<'n>(
    code: &[u8],
    parser: &mut tree_sitter::Parser,
) -> Result<HashMap<Cow<'n, str>, Vec<Range<usize>>>, Error> {
    let tree = parser.parse(code, None).ok_or(Error::TreeSitterParsing)?;
    let (named, unnamed): (Vec<_>, Vec<_>) = tree
        .root_node()
        .iter_depth_first()
        // .filter(|it| !it.is_error())
        .partition(|it| it.is_named());
    let blacklist: HashSet<_> = unnamed.into_iter().map(|it| it.kind()).collect();

    let from_tree = named.into_iter().filter(|it| !it.is_error()).map(|it| {
        let kind = it.kind();
        (Cow::Borrowed(kind), it.byte_range())
    });

    let graph = tree_to_dot_graph(tree.clone())?;
    let graph_terminals = dot_graph_to_terminals(graph)?
        .into_iter()
        .filter(|(k, _)| !blacklist.contains(k.as_str()) && k != "ERROR" && k != "_ERROR")
        .map(|(k, v)| (Cow::Owned(k), v));

    Ok(from_tree.chain(graph_terminals).into_group_map())
}

fn tree_to_dot_graph(tree: tree_sitter::Tree) -> Result<Graph, Error> {
    let (mut pipe_reader, pipe_writer) = os_pipe::pipe()?;
    let plotting_thread = std::thread::spawn(move || tree.print_dot_graph(&pipe_writer));
    let buffer = {
        let mut buf = Vec::new();
        pipe_reader.read_to_end(&mut buf)?;
        buf.shrink_to_fit();
        buf
    };
    plotting_thread.join().map_err(|_| Error::PlotGraphPanic)?;
    let dot_code = String::from_utf8(buffer).map_err(|err| Error::Utf8Parsing {
        what: "Plotted dot graph",
        err,
    })?;
    graphviz_rust::parse(&dot_code).map_err(Error::DotGraphParsing)
}

fn dot_graph_to_terminals(graph: Graph) -> Result<Vec<(String, Range<usize>)>, Error> {
    let Graph::DiGraph { stmts, .. } = graph else {
        return Err(Error::DotGraphFormatMismatch("Expected a digraph"));
    };
    let (nodes, edges): (Vec<_>, Vec<_>) = stmts
        .into_iter()
        .filter(|it| matches!(it, Stmt::Node(_) | Stmt::Edge(_)))
        .partition_map(|it| match it {
            Stmt::Node(node) => Either::Left(node),
            Stmt::Edge(edge) => Either::Right(edge),
            _ => unreachable!("We filtered out other variants above."),
        });
    let non_terminals_ids: HashSet<_> = edges
        .into_iter()
        .map(|edge| {
            if let Edge {
                ty: EdgeTy::Pair(Vertex::N(NodeId(Id::Plain(source), _)), _),
                ..
            } = edge
            {
                Ok(source)
            } else {
                Err(Error::DotGraphFormatMismatch("Edge node id format"))
            }
        })
        .try_collect()?;

    nodes
        .into_iter()
        .map(try_extract_id_and_attr)
        .filter_ok(|(id, _)| !non_terminals_ids.contains(id))
        .map(|result| {
            result.and_then(|(_, attributes)| {
                let mut label = None;
                let mut tooltip = None;
                for Attribute(attr, value) in attributes {
                    if label.is_some() && tooltip.is_some() {
                        break;
                    }
                    match attr {
                        Id::Plain(attr) if attr == "label" => {
                            label = Some(try_extract_escaped(value)?);
                        }
                        Id::Plain(attr) if attr == "tooltip" => {
                            tooltip = Some(try_extract_escaped(value)?);
                        }
                        _ => {}
                    }
                }
                let label = label.ok_or(Error::DotGraphFormatMismatch("No label attribute"))?;
                let tooltip =
                    tooltip.ok_or(Error::DotGraphFormatMismatch("No tooltip attribute"))?;
                let range = node_range(&tooltip).expect("fuck");
                Ok((label, range))
            })
        })
        .try_collect()
}

fn try_extract_escaped(value: Id) -> Result<String, Error> {
    if let Id::Escaped(val) = value {
        if val.lines().count() < 2 {
            if let Ok(serde_json::Value::String(value)) = serde_json::Value::from_str(&val) {
                return Ok(value);
            }
        } else {
            return Ok(val.trim_matches('"').to_owned());
        }
    }
    Err(Error::DotGraphFormatMismatch("Expected escaped string"))
}

fn try_extract_id_and_attr(node: Node) -> Result<(String, Vec<Attribute>), Error> {
    if let Node {
        id: NodeId(Id::Plain(id), _),
        attributes,
    } = node
    {
        Ok((id, attributes))
    } else {
        Err(Error::DotGraphFormatMismatch("Node id format"))
    }
}

fn node_range(tooltip: &str) -> Result<Range<usize>, Error> {
    let line = tooltip
        .lines()
        .find(|it| it.starts_with("range: "))
        .map(|line| line.trim())
        .ok_or(Error::DotGraphFormatMismatch("Cannot find range line"))?;
    let (start, end) = line
        .trim_start_matches("range: ")
        .split_once(" - ")
        .ok_or(Error::DotGraphFormatMismatch("Invalid range line"))?;
    let start = start.trim().parse()?;
    let end = end.trim().parse()?;
    Ok(start..end)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Fail to parse the code with tree-sitter")]
    TreeSitterParsing,

    #[error("IO Error: {_0}")]
    IO(#[from] std::io::Error),

    #[error("Panics when plotting dot graph")]
    PlotGraphPanic,

    #[error("Fail to parse UTF-8 of {what}: {err:?}")]
    Utf8Parsing {
        what: &'static str,
        err: FromUtf8Error,
    },

    #[error("Fail to parse the grnerated dot graph code: {_0}")]
    DotGraphParsing(String),

    #[error("Holy shit, the dot graph format mismatch: {_0}")]
    DotGraphFormatMismatch(&'static str),

    #[error("Fail to parse ranges: {_0}")]
    ParsingRange(#[from] ParseIntError),
}

#[cfg(test)]
mod test {

    use super::*;

    const C_CODE: &str = r#"
    #include <stdio.h>
    #include <stdlib.h>

    const int x = 42;

    int main(int argc, char *argv[] "\n") {
        printf("Hello, world!\n");
        return 0;
    }
    "#;

    #[test]
    fn test_extract_derivation_fragments() {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_c::LANGUAGE.into();
        parser.set_language(&lang).unwrap();
        let fragments = extract_derivation_fragments(C_CODE.as_bytes(), &mut parser).unwrap();
        for key in [
            "translation_unit",
            "function_definition",
            "declaration",
            "expression_statement",
            "call_expression",
            "string_literal",
            "number_literal",
            "preproc_include_token2",
        ] {
            assert!(fragments.contains_key(key), "{key} not found");
        }
    }
}
