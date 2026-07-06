use tree_sitter::{Parser, Query, QueryCursor, Range, StreamingIterator};

const SOURCE_FILE: &str = include_str!("../misc/error.cmake");

#[derive(Debug, PartialEq, Eq, Clone)]
struct HighLightNode {
    range: Range,
    identifier: String,
    names: Vec<String>,
    children: Vec<HighLightNode>,
}

trait RangeContain {
    fn contain(&self, other: &Self) -> bool;
}

impl RangeContain for Range {
    fn contain(&self, other: &Self) -> bool {
        self.start_byte <= other.start_byte && self.end_byte >= other.end_byte
    }
}

impl RangeContain for HighLightNode {
    fn contain(&self, other: &Self) -> bool {
        self.range.contain(&other.range)
    }
}

impl Ord for HighLightNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.range.start_byte >= other.range.end_byte {
            return std::cmp::Ordering::Greater;
        }
        if self.range.end_byte <= other.range.start_byte {
            return std::cmp::Ordering::Less;
        }
        std::cmp::Ordering::Equal
    }
}

impl HighLightNode {
    fn insert_node(&mut self, range: Range, highlight: &str, identifier: &str) -> bool {
        assert!(self.children.is_sorted());
        if let Some(hnode) = self.children.iter_mut().find(|hnode| hnode.range == range) {
            hnode.names.push(highlight.to_owned());
            return false;
        }
        if let Some(hnode) = self
            .children
            .iter_mut()
            .find(|hnode| hnode.range.contain(&range))
        {
            return hnode.insert_node(range, highlight, identifier);
        }
        self.children.push(HighLightNode {
            range,
            identifier: identifier.to_owned(),
            names: vec![highlight.to_owned()],
            children: Vec::new(),
        });
        self.children.sort();
        true
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct HighLightNodeContainer {
    nodes: Vec<HighLightNode>,
}

impl HighLightNodeContainer {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }
    fn insert_node(&mut self, range: Range, highlight: &str, identifier: &str) {
        assert!(self.nodes.is_sorted());
        if let Some(hnode) = self.nodes.iter_mut().find(|hnode| hnode.range == range) {
            hnode.names.push(highlight.to_owned());
            return;
        }
        if let Some(hnode) = self
            .nodes
            .iter_mut()
            .find(|hnode| hnode.range.contain(&range))
        {
            hnode.insert_node(range, highlight, identifier);
            return;
        }
        self.nodes.push(HighLightNode {
            range,
            identifier: identifier.to_owned(),
            names: vec![highlight.to_owned()],
            children: Vec::new(),
        });
        self.nodes.sort();
    }
}

impl PartialOrd for HighLightNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.range.start_byte >= other.range.end_byte {
            return Some(std::cmp::Ordering::Greater);
        }
        if self.range.end_byte <= other.range.start_byte {
            return Some(std::cmp::Ordering::Less);
        }
        if self.range == other.range {
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

            container.insert_node(e.node.range(), names[e.index as usize], e.node.kind());
        }
    }
    println!("{container:?}");
}
