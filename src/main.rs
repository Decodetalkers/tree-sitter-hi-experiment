use std::sync::LazyLock;

use lsp_types::{SemanticToken, SemanticTokenTypes};
use tree_sitter::{Node, Parser, Query, QueryCursor, Range, StreamingIterator};

const SOURCE_FILE: &str = include_str!("../misc/error.cmake");

pub const LEGEND_TYPE: &[SemanticTokenTypes] = &[
    SemanticTokenTypes::Function,
    SemanticTokenTypes::Method,
    SemanticTokenTypes::Variable,
    SemanticTokenTypes::String,
    SemanticTokenTypes::Comment,
    SemanticTokenTypes::Number,
    SemanticTokenTypes::Keyword,
    SemanticTokenTypes::Operator,
    SemanticTokenTypes::Parameter,
];

fn get_token_position(tokentype: SemanticTokenTypes) -> u32 {
    LEGEND_TYPE
        .iter()
        .position(|data| *data == tokentype)
        .unwrap() as u32
}

trait GetToken {
    fn hl_token(&self, source: &str) -> Option<SemanticTokenTypes>;
    fn hl_token_index(&self, source: &str) -> Option<u32> {
        Some(get_token_position(self.hl_token(source)?))
    }
}

static NUMBERREGEX: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\d+(?:\.+\d*)?").unwrap());

#[derive(Debug, PartialEq, Eq)]
struct HighLightNode<'a> {
    node: Node<'a>,
    highlights: Vec<&'a str>,
    children: Vec<HighLightNode<'a>>,
}

impl<'a> GetToken for HighLightNode<'a> {
    fn hl_token(&self, source: &str) -> Option<SemanticTokenTypes> {
        if self.highlights.contains(&"function") {
            return Some(SemanticTokenTypes::Function);
        }
        if self.highlights.contains(&"string") {
            return Some(SemanticTokenTypes::String);
        }
        if self.highlights.contains(&"comment") && self.highlights.contains(&"spell") {
            return Some(SemanticTokenTypes::Comment);
        }
        if self.highlights.contains(&"constant") {
            if self
                .node
                .utf8_text(source.as_bytes())
                .is_ok_and(|txt| NUMBERREGEX.is_match(txt))
            {
                return Some(SemanticTokenTypes::Number);
            }
            return Some(SemanticTokenTypes::Keyword);
        }
        if self.highlights.iter().any(|hl| hl.starts_with("keyword")) {
            return Some(SemanticTokenTypes::Keyword);
        }
        if self
            .highlights
            .iter()
            .any(|hl| hl.starts_with("punctuation"))
        {
            return Some(SemanticTokenTypes::Operator);
        }
        if self.highlights.contains(&"variable") {
            return Some(SemanticTokenTypes::Variable);
        }
        None
    }
}

trait RangeContain {
    fn contain(&self, other: &Self) -> bool;
}

impl RangeContain for Range {
    fn contain(&self, other: &Self) -> bool {
        self.start_byte <= other.start_byte && self.end_byte >= other.end_byte
    }
}

impl<'a> RangeContain for HighLightNode<'a> {
    fn contain(&self, other: &Self) -> bool {
        self.node.range().contain(&other.node.range())
    }
}

impl<'a> Ord for HighLightNode<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.range().start_byte >= other.range().end_byte {
            return std::cmp::Ordering::Greater;
        }
        if self.range().end_byte <= other.range().start_byte {
            return std::cmp::Ordering::Less;
        }
        std::cmp::Ordering::Equal
    }
}

impl<'a> HighLightNode<'a> {
    fn range(&self) -> Range {
        self.node.range()
    }
    fn insert_node(&mut self, node: Node<'a>, highlight: &'a str) -> bool {
        assert!(self.children.is_sorted());
        if let Some(hnode) = self
            .children
            .iter_mut()
            .find(|hnode| hnode.range() == node.range())
        {
            hnode.highlights.push(highlight);
            return false;
        }
        if let Some(hnode) = self
            .children
            .iter_mut()
            .find(|hnode| hnode.range().contain(&node.range()))
        {
            return hnode.insert_node(node, highlight);
        }
        self.children.push(HighLightNode {
            node,
            highlights: vec![highlight],
            children: Vec::new(),
        });
        self.children.sort();
        true
    }
}

#[derive(Debug, PartialEq, Eq)]
struct HighLightNodeContainer<'a> {
    nodes: Vec<HighLightNode<'a>>,
}

impl<'a> HighLightNodeContainer<'a> {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }
    fn insert_node(&mut self, node: Node<'a>, highlight: &'a str) {
        assert!(self.nodes.is_sorted());
        if let Some(hnode) = self
            .nodes
            .iter_mut()
            .find(|hnode| hnode.range() == node.range())
        {
            hnode.highlights.push(highlight);
            return;
        }
        if let Some(hnode) = self
            .nodes
            .iter_mut()
            .find(|hnode| hnode.range().contain(&node.range()))
        {
            hnode.insert_node(node, highlight);
            return;
        }
        self.nodes.push(HighLightNode {
            node,
            highlights: vec![highlight],
            children: Vec::new(),
        });
        self.nodes.sort();
    }
}

impl<'a> PartialOrd for HighLightNode<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.range().start_byte >= other.range().end_byte {
            return Some(std::cmp::Ordering::Greater);
        }
        if self.range().end_byte <= other.range().start_byte {
            return Some(std::cmp::Ordering::Less);
        }
        if self.range() == other.range() {
            return Some(std::cmp::Ordering::Equal);
        }
        None
    }
}

fn main() {
    let query_source = tree_sitter_cmake::HIGHLIGHTS_QUERY;
    let language: tree_sitter::Language = tree_sitter_cmake::LANGUAGE.into();
    let query = Query::new(&language, query_source).unwrap();

    let mut parser = Parser::new();
    parser.set_language(&language).unwrap();
    let tree = parser.parse(SOURCE_FILE, None).unwrap();
    let node = tree.root_node();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, node, SOURCE_FILE.as_bytes());

    let mut container = HighLightNodeContainer::new();
    let names = query.capture_names();
    while let Some(m) = matches.next() {
        for e in m.captures {
            container.insert_node(e.node, names[e.index as usize]);
        }
    }
    println!("{container:?}");
}
