use crate::ast;

fn generate_clauses_for_leaf_contents<'a>(
    leaf_contents: &ast::InlineNodeList<'a>,
    clauses: &mut ClauseList<'a>,
    idx: &mut usize,
    source_language: &pulldown_cmark::CowStr<'a>,
) {
    for node in leaf_contents {
        if let ast::InlineNode::Surrounding { tag, contents } = node {
            if *tag == ast::SurroundingInlineTag::Custom(crate::sentence_segment::SENTENCE_TAG) {
                let cur_idx = *idx;
                *idx += 1;
                let clause = Clause {
                    clause_contents: contents.clone(),
                    clause_idx: cur_idx,
                    clause_translations: vec![(source_language.clone(), contents.clone())],
                };
                clauses.push(clause);
            }
        }
    }
}

fn generate_clauses_for_block_node<'a>(
    block: &ast::BlockNode<'a>,
    clauses: &mut ClauseList<'a>,
    idx: &mut usize,
    source_language: &pulldown_cmark::CowStr<'a>,
) {
    match block {
        ast::BlockNode::Container { children, .. } => {
            for block in children.iter() {
                generate_clauses_for_block_node(block, clauses, idx, source_language);
            }
        }
        ast::BlockNode::Leaf { contents, .. } => {
            generate_clauses_for_leaf_contents(contents, clauses, idx, source_language);
        }
        ast::BlockNode::Markup { .. } => {
            // do nothing
        }
    }
}

fn generate_clauses_for_out_of_band_node<'a>(
    oob: &ast::OutOfBandNode<'a>,
    clauses: &mut ClauseList<'a>,
    idx: &mut usize,
    source_language: &pulldown_cmark::CowStr<'a>,
) {
    match oob {
        ast::OutOfBandNode::OutOfBandContainer { children, .. } => {
            for block in children.iter() {
                generate_clauses_for_block_node(block, clauses, idx, source_language);
            }
        }
        ast::OutOfBandNode::OutOfBandLeaf { contents, .. } => {
            generate_clauses_for_leaf_contents(contents, clauses, idx, source_language);
        }
    }
}

impl<'a> ast::Document<'a> {
    pub fn extract_clause_list(
        &self,
        source_language: &pulldown_cmark::CowStr<'a>,
    ) -> DocumentClauseList<'a> {
        let mut clause_list = DocumentClauseList {
            clauses: Vec::new(),
        };
        let mut clause_idx = 1;
        for block in self.blocks.iter() {
            generate_clauses_for_block_node(
                block,
                &mut clause_list.clauses,
                &mut clause_idx,
                source_language,
            );
        }

        for outofband in self.outofbands.iter() {
            generate_clauses_for_out_of_band_node(
                outofband,
                &mut clause_list.clauses,
                &mut clause_idx,
                source_language,
            );
        }

        clause_list
    }
}

pub struct DocumentClauseList<'a> {
    pub(crate) clauses: ClauseList<'a>,
}

type ClauseList<'a> = Vec<Clause<'a>>;

pub struct Clause<'a> {
    pub(crate) clause_contents: ast::InlineNodeList<'a>,
    pub(crate) clause_idx: usize,
    pub(crate) clause_translations: Vec<(pulldown_cmark::CowStr<'a>, ast::InlineNodeList<'a>)>,
}
