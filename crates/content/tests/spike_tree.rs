//! **Spike, not production.** What a real tree costs and buys, measured.
//!
//! `Content` stores lines flat and *derives* the block tree from each line's
//! `containers` path plus contiguity. `model.rs` states the reason: "the line
//! tree is derived from this flat list plus each line's `containers` path, never
//! stored, so a split/join is a single-char edit with no paragraph identity to
//! reconcile."
//!
//! The cost of that choice is that sibling identity has nowhere to live, which
//! is #1357/#1359. This file builds the tree the model declines to store, runs
//! the corpus through both conversions, and measures the gap.
//!
//! Two questions, two directions:
//!
//! - `flat → tree → flat` — does a tree lose anything the flat form holds? If
//!   this is the identity over the corpus, the tree is at least as expressive.
//! - `tree → flat → tree` — what does the flat form lose? Every tree that does
//!   not survive is a document the model cannot store, and the set of them is
//!   the exact scope of the adjacency problem.
//!
//! The text, marks and islands are deliberately *not* moved into the tree: they
//! stay in the flat `Content` and the tree indexes into `lines`. That is the
//! cheap variant. See `SPIKE_TREE.md` for what the expensive one costs.

use quillmark_content::model::{Container, Content, Line, LineKind};
use quillmark_content::{from_markdown, to_markdown};

// ─── The tree ───────────────────────────────────────────────────────────────

/// A block node. Interior nodes are the containers; `Block` is a leaf holding
/// the flat line indices of one `continues`-joined run.
///
/// What is *gone* relative to the flat encoding, and gone by construction:
///
/// - `ordinal` — an item's index is its position in `List::items`.
/// - any sibling discriminator — two sibling nodes are two nodes.
/// - the depth-prefix comparison every consumer re-derives — children are
///   children.
/// - the "container path jumps from depth 3 to depth 1" state, which the flat
///   form can spell and nothing rejects.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    List {
        ordered: bool,
        start: u64,
        items: Vec<Vec<Node>>,
    },
    Quote(Vec<Node>),
    Unknown {
        tag: String,
        attrs: serde_json::Value,
        children: Vec<Node>,
    },
    /// A leaf block: its kind plus the flat lines it spans. The first has
    /// `continues: false`, the rest `true`.
    Block { kind: LineKind, lines: Vec<usize> },
}

/// Flat → tree. Reads the container paths the way the model documents: a line
/// belongs to the container its path names, and *contiguity* decides whether
/// two adjacent equal paths are one container or two. This function is
/// therefore where the flat form's lossiness is realized — it has no choice but
/// to weld two adjacent equal paths, because that is all the data says.
fn to_tree(rt: &Content) -> Vec<Node> {
    to_tree_with(rt, Reader::Best)
}

/// How hard the flat → tree reader tries. The gap between these two *is*
/// #1359: `Shipped` is the rule `emit.rs::list_run_end` and `census` use on
/// `main`, `Best` additionally reads an `ordinal` decrease as the next list.
#[derive(Clone, Copy, PartialEq)]
enum Reader {
    Shipped,
    Best,
}

fn to_tree_with(rt: &Content, reader: Reader) -> Vec<Node> {
    fn build(rt: &Content, range: std::ops::Range<usize>, depth: usize, reader: Reader) -> Vec<Node> {
        let mut out = Vec::new();
        let mut i = range.start;
        while i < range.end {
            let path = &rt.lines[i].containers;
            if path.len() > depth {
                let key = &path[depth];
                // Two different grouping rules live at this one depth, and
                // which applies depends on the arm. A `Quote`/`Unknown` run is
                // whole-key equality. A *list* run is like-shapedness — items
                // differ in `ordinal` and still belong to one list — so the
                // list boundary has to be recovered by a second, weaker rule.
                // That split is the flat encoding's whole difficulty in one
                // place: the emitters, `census` and the TS codec each
                // re-derive it, and #1359 is what happens when one of them
                // picks a different weakening.
                let mut j = i + 1;
                let mut prev = key;
                while j < range.end
                    && rt.lines[j].containers.len() > depth
                    && same_container(prev, &rt.lines[j].containers[depth], reader)
                {
                    prev = &rt.lines[j].containers[depth];
                    j += 1;
                }
                out.push(container_node(rt, i..j, depth, key, reader));
                i = j;
            } else {
                // A leaf block: this line plus every `continues` line after it.
                let mut j = i + 1;
                while j < range.end
                    && rt.lines[j].containers.len() == depth
                    && rt.lines[j].continues
                {
                    j += 1;
                }
                out.push(Node::Block {
                    kind: rt.lines[i].kind.clone(),
                    lines: (i..j).collect(),
                });
                i = j;
            }
        }
        out
    }

    /// Same container *instance*, as every consumer must decide it.
    ///
    /// Whole-key equality for a container with no shape of its own. For a list
    /// item the rule must be *weakened* — items differ in `ordinal` and still
    /// belong to one list — and how far to weaken it is exactly the question
    /// four consumers answered separately. `Shipped` drops `ordinal` outright;
    /// `Best` keeps a decrease as the boundary it is.
    fn same_container(prev: &Container, next: &Container, reader: Reader) -> bool {
        match (prev, next) {
            (
                Container::ListItem {
                    ordered: o1,
                    start: s1,
                    ordinal: a,
                },
                Container::ListItem {
                    ordered: o2,
                    start: s2,
                    ordinal: b,
                },
            ) => o1 == o2 && s1 == s2 && (reader == Reader::Shipped || b >= a),
            _ => prev == next,
        }
    }

    fn container_node(
        rt: &Content,
        range: std::ops::Range<usize>,
        depth: usize,
        key: &Container,
        reader: Reader,
    ) -> Node {
        match key {
            Container::ListItem { ordered, start, .. } => {
                // One `emit_container`-shaped regrouping: the run that shares a
                // whole path is one *item*; the surrounding run of like-shaped
                // items is the list. Both re-derivations the flat form forces.
                let mut items = Vec::new();
                let mut k = range.start;
                while k < range.end {
                    let item_key = rt.lines[k].containers[depth].clone();
                    let mut m = k + 1;
                    while m < range.end && rt.lines[m].containers[depth] == item_key {
                        m += 1;
                    }
                    items.push(build(rt, k..m, depth + 1, reader));
                    k = m;
                }
                Node::List {
                    ordered: *ordered,
                    start: *start,
                    items,
                }
            }
            Container::Quote => Node::Quote(build(rt, range, depth + 1, reader)),
            Container::Unknown { tag, attrs } => Node::Unknown {
                tag: tag.clone(),
                attrs: attrs.clone(),
                children: build(rt, range, depth + 1, reader),
            },
            _ => Node::Quote(build(rt, range, depth + 1, reader)),
        }
    }

    build(rt, 0..rt.lines.len(), 0, reader)
}

/// Tree → flat. Re-derives every container path, taking each item's `ordinal`
/// from its position. Text is carried over unchanged: this rebuilds `lines`
/// only, which is the whole structural surface.
fn from_tree(nodes: &[Node], src: &Content) -> Content {
    fn walk(nodes: &[Node], src: &Content, path: &mut Vec<Container>, out: &mut Vec<Line>) {
        for node in nodes {
            match node {
                Node::List {
                    ordered,
                    start,
                    items,
                } => {
                    for (ordinal, item) in items.iter().enumerate() {
                        path.push(Container::ListItem {
                            ordered: *ordered,
                            start: *start,
                            ordinal: ordinal as u64,
                        });
                        walk(item, src, path, out);
                        path.pop();
                    }
                }
                Node::Quote(children) => {
                    path.push(Container::Quote);
                    walk(children, src, path, out);
                    path.pop();
                }
                Node::Unknown {
                    tag,
                    attrs,
                    children,
                } => {
                    path.push(Container::Unknown {
                        tag: tag.clone(),
                        attrs: attrs.clone(),
                    });
                    walk(children, src, path, out);
                    path.pop();
                }
                Node::Block { kind, lines } => {
                    for (n, ix) in lines.iter().enumerate() {
                        out.push(
                            Line::new(kind.clone())
                                .with_containers(path.clone())
                                .with_continues(n > 0 && src.lines[*ix].continues),
                        );
                    }
                }
            }
        }
    }
    let mut lines = Vec::new();
    walk(nodes, src, &mut Vec::new(), &mut lines);
    let mut rt = Content::new(src.text.clone(), lines)
        .with_marks(src.marks.clone())
        .with_islands(src.islands.clone());
    rt.normalize();
    rt
}

// ─── Direction 1: does a tree lose anything the flat form holds? ────────────

/// Every construct the codecs know, plus the shapes the repo's own tests
/// accumulated as edge cases.
const CORPUS: &[&str] = &[
    "hello",
    "# h1\n\n## h2\n\npara",
    "- a\n- b\n- c",
    "1. a\n2. b",
    "3. a\n4. b",
    "- a\n\n  second para of a\n\n- b",
    "> quoted\n>\n> two paragraphs",
    "> outer\n>\n> > nested",
    "- outer\n  - inner\n  - inner2\n- outer2",
    "- a\n  > quote in item\n- b",
    "```rust\nfn main() {}\nlet x = 1;\n```",
    "para\n\n---\n\npara2",
    "line one\\\nline two",
    "- one\\\ntwo\n- three",
    "**bold** and _em_ and `code` and [l](u)",
    "| a | b |\n|---|---|\n| 1 | 2 |",
    "![alt](img.png)",
    "text with ![inline](i.png) image",
    "- a\n\n<!-- -->\n\n- b",
    "* a\n\n+ b",
    "1. a\n2. b\n\n<!-- -->\n\n1. c\n2. d",
    "> a\n\n> b",
    "- ***",
    "1. ---",
    "- one\n\n  ---",
    "",
    "- \n- x",
    "> ",
    "1. a\n   1. b\n2. c",
    "para\n\n- list\n\npara2\n\n- list2",
];

fn corpus() -> Vec<(&'static str, Content)> {
    CORPUS
        .iter()
        .map(|md| (*md, from_markdown(md).expect("imports")))
        .collect()
}

#[test]
fn flat_to_tree_to_flat_is_the_identity() {
    for (md, rt) in corpus() {
        let round = from_tree(&to_tree(&rt), &rt);
        assert_eq!(
            round, rt,
            "the tree lost something the flat form holds\n  source: {md:?}"
        );
    }
}

/// The tree carries the container path faithfully enough that the *markdown*
/// projection is unchanged too — the structural conversion is not quietly
/// leaning on `normalize` to repair it.
#[test]
fn the_tree_preserves_the_markdown_projection() {
    for (md, rt) in corpus() {
        let round = from_tree(&to_tree(&rt), &rt);
        assert_eq!(to_markdown(&round), to_markdown(&rt), "source: {md:?}");
    }
}

// ─── Direction 2: what does the flat form lose? ─────────────────────────────

fn para(ix: usize) -> Node {
    Node::Block {
        kind: LineKind::Para,
        lines: vec![ix],
    }
}

fn ul(items: Vec<Vec<Node>>) -> Node {
    Node::List {
        ordered: false,
        start: 1,
        items,
    }
}

fn ol(start: u64, items: Vec<Vec<Node>>) -> Node {
    Node::List {
        ordered: true,
        start,
        items,
    }
}

/// The gap between the two readers, counted over the enumerated space: how
/// many documents `main` loses that a maximally clever flat reader keeps.
/// That difference is the whole of #1359; what `Best` still loses is what no
/// reader can recover, and is the case for storing a tree.
#[test]
fn measure_the_gap_between_the_readers() {
    let space = enumerate_space();
    let (mut shipped_lost, mut best_lost) = (0usize, 0usize);
    let mut irreducible: Vec<String> = Vec::new();
    for (name, tree, lines) in &space {
        let flat = from_tree(tree, &scaffold(*lines));
        if &to_tree_with(&flat, Reader::Shipped) != tree {
            shipped_lost += 1;
        }
        if &to_tree_with(&flat, Reader::Best) != tree {
            best_lost += 1;
            irreducible.push(name.clone());
        }
    }
    let n = space.len();
    eprintln!(
        "\n  trees enumerated:            {n}\
         \n  lost by the shipped reader:  {shipped_lost} ({:.1}%)\
         \n  lost by the best reader:     {best_lost} ({:.1}%)\
         \n  recoverable by reading the ordinal decrease (#1359): {}\
         \n  irreducible without a stored boundary:               {best_lost}",
        100.0 * shipped_lost as f64 / n as f64,
        100.0 * best_lost as f64 / n as f64,
        shipped_lost - best_lost,
    );
    let mut sample: Vec<&String> = irreducible.iter().take(8).collect();
    sample.sort();
    for x in sample {
        eprintln!("    irreducible: {x}");
    }
    assert!(
        best_lost < shipped_lost,
        "reading the decrease should recover documents"
    );
    assert!(
        best_lost > 0,
        "and should still not recover all of them: that residue is the case for a tree"
    );
}

/// A flat content with `n` single-char lines for a tree to index into.
fn scaffold(n: usize) -> Content {
    let text = (0..n).map(|i| ((b'a' + i as u8) as char).to_string()).collect::<Vec<_>>().join("\n");
    let lines = (0..n).map(|_| Line::new(LineKind::Para)).collect();
    Content::new(text, lines)
}

/// Round-trip a hand-built tree through the flat encoding and back.
fn survives(tree: &[Node], lines: usize) -> bool {
    let src = scaffold(lines);
    let flat = from_tree(tree, &src);
    &to_tree(&flat) == tree
}

/// The exact set the flat encoding cannot hold. Each of these is a document a
/// tree spells without effort and `Content` reads back as something else.
#[test]
fn the_flat_encoding_loses_adjacent_siblings() {
    // Two one-item lists → one item of two paragraphs. #1357's headline case.
    assert!(!survives(
        &[ul(vec![vec![para(0)]]), ul(vec![vec![para(1)]])],
        2
    ));

    // A one-item list then a two-item list: no ordinal below 0 to restart to.
    assert!(!survives(
        &[ul(vec![vec![para(0)]]), ul(vec![vec![para(1)], vec![para(2)]])],
        3
    ));

    // Two adjacent quotes → one two-paragraph quote.
    assert!(!survives(&[Node::Quote(vec![para(0)]), Node::Quote(vec![para(1)])], 2));

    // Two adjacent unknown containers of equal (tag, attrs).
    let unk = |ix| Node::Unknown {
        tag: "callout".into(),
        attrs: serde_json::json!({"level": 1}),
        children: vec![para(ix)],
    };
    assert!(!survives(&[unk(0), unk(1)], 2));

    // Two adjacent ordered lists of two items each. This one the *model* keeps
    // (the ordinal decrease carries it) — and both projections then drop it,
    // which is #1359's defect 2 and 3.
    let two_ol = [
        ol(1, vec![vec![para(0)], vec![para(1)]]),
        ol(1, vec![vec![para(2)], vec![para(3)]]),
    ];
    assert!(survives(&two_ol, 4), "the model keeps this one");
    let flat = from_tree(&two_ol, &scaffold(4));
    let reimported = from_markdown(&to_markdown(&flat)).unwrap();
    assert_ne!(
        to_tree(&reimported),
        two_ol.to_vec(),
        "…but the markdown projection does not"
    );
}

/// The shapes that do survive, pinned so the boundary is exact rather than
/// folklore: it is *adjacency of equal shape* that is lost, nothing wider.
#[test]
fn everything_not_adjacent_and_equal_survives() {
    let cases: Vec<(&str, Vec<Node>, usize)> = vec![
        ("one list, two items", vec![ul(vec![vec![para(0)], vec![para(1)]])], 2),
        (
            "one item, two paragraphs",
            vec![ul(vec![vec![para(0), para(1)]])],
            2,
        ),
        (
            "lists of differing kind",
            vec![ul(vec![vec![para(0)]]), ol(1, vec![vec![para(1)]])],
            2,
        ),
        (
            "separated by a paragraph",
            vec![ul(vec![vec![para(0)]]), para(1), ul(vec![vec![para(2)]])],
            3,
        ),
        (
            "nested: two inner lists under one item, split by a paragraph",
            vec![ul(vec![vec![
                ul(vec![vec![para(0)]]),
                para(1),
                ul(vec![vec![para(2)]]),
            ]])],
            3,
        ),
        (
            "quote inside a list item",
            vec![ul(vec![vec![Node::Quote(vec![para(0)])], vec![para(1)]])],
            2,
        ),
        (
            "three levels",
            vec![ul(vec![vec![Node::Quote(vec![ol(
                2,
                vec![vec![para(0)]],
            )])]])],
            1,
        ),
    ];
    for (name, tree, lines) in cases {
        assert!(survives(&tree, lines), "should survive: {name}");
    }
}

// ─── How much of the space is lost ──────────────────────────────────────────

/// Enumerate every tree of `n` top-level container nodes drawn from a small
/// alphabet, and count how many the flat encoding can hold. A number rather
/// than an anecdote: this is the fraction of documents `Content` can store.
/// Every ordered pair and triple over a small alphabet of container shapes.
fn enumerate_space() -> Vec<(String, Vec<Node>, usize)> {
    // A small alphabet of one-line container shapes, each carrying `k` items.
    fn shapes(ix: &mut usize) -> Vec<(String, Node, usize)> {
        let mut out = Vec::new();
        for (name, n_items) in [("ul", 1usize), ("ul", 2), ("ol", 1), ("ol", 2)] {
            let items: Vec<Vec<Node>> = (0..n_items)
                .map(|d| vec![para(*ix + d)])
                .collect();
            let node = if name == "ul" {
                ul(items)
            } else {
                ol(1, items)
            };
            out.push((format!("{name}{n_items}"), node, n_items));
        }
        out.push(("quote".into(), Node::Quote(vec![para(*ix)]), 1));
        out.push(("para".into(), para(*ix), 1));
        out
    }

    let mut space = Vec::new();
    for depth in 2..=3 {
        let mut stack: Vec<(Vec<String>, Vec<Node>, usize)> = vec![(vec![], vec![], 0)];
        for _ in 0..depth {
            let mut next = Vec::new();
            for (names, nodes, used) in &stack {
                let mut ix = *used;
                for (name, node, n) in shapes(&mut ix) {
                    let mut names2 = names.clone();
                    names2.push(name);
                    let mut nodes2 = nodes.clone();
                    nodes2.push(node);
                    next.push((names2, nodes2, used + n));
                }
            }
            stack = next;
        }
        for (names, nodes, used) in stack {
            space.push((names.join(" + "), nodes, used.max(1)));
        }
    }
    space
}

// ─── What the tree costs: node identity under a split ───────────────────────

/// The property `model.rs` cites for storing lines flat: a paragraph split is
/// one `\n` insertion, and no node identity has to be reconciled.
///
/// Spelled out on both sides. Flat: splice the text, the line count follows.
/// Tree: locate the node, partition its lines, replace one child with two — and
/// then answer the question the flat form never asks, which is *which of the
/// two is the original node*. Anchors, comments and any future per-node handle
/// all hang off that answer.
#[test]
fn a_split_costs_a_node_identity_decision() {
    let rt = from_markdown("- alpha\n\n  beta\n- gamma").unwrap();
    let tree = to_tree(&rt);

    // The flat split: one `\n`, and the structure follows from the text.
    let mut flat_split = rt.clone();
    flat_split
        .apply_line_ops(&[quillmark_content::LineOp::Split { at: 2 }])
        .expect("split applies");
    assert_eq!(flat_split.lines.len(), rt.lines.len() + 1);
    // Every line's container path is untouched but for the new one, which
    // copies its neighbour: no identity was minted or destroyed.
    assert_eq!(flat_split.lines[0].containers, rt.lines[0].containers);
    assert_eq!(flat_split.lines[1].containers, rt.lines[0].containers);

    // The tree split, written out to show what the flat one does not have to
    // decide. Splitting the first block of item 0 into two blocks means
    // replacing one child with two — and nothing in the model says whether a
    // handle on the original block follows the head or the tail.
    let Node::List { items, .. } = &tree[0] else {
        panic!("expected a list")
    };
    let item0 = &items[0];
    assert_eq!(item0.len(), 2, "alpha and beta are two blocks of one item");
    let Node::Block { kind, lines } = &item0[0] else {
        panic!("expected a block")
    };
    let (head, tail) = (
        Node::Block {
            kind: kind.clone(),
            lines: lines.clone(),
        },
        Node::Block {
            kind: kind.clone(),
            lines: vec![lines.len()],
        },
    );
    assert_ne!(head, tail);
    // Both are new values; the original is gone. In the flat encoding there was
    // never an object to lose.
}
